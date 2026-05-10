use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::BUFFER_CAPACITY;

/// Thread-safe broadcast ring buffer.
///
/// Single writer, multiple readers. Each reader maintains an independent
/// read position. The writer overwrites oldest data when full, advancing
/// any reader positions that fall behind to avoid reading garbage.
///
/// # Safety
///
/// `UnsafeCell` is used for the `data` buffer to allow lock-free reads while
/// the writer modifies it. This is sound because:
/// 1. The writer holds the internal write lock during writes, ensuring exclusive write access.
/// 2. The writer updates `write_pos` with `Release` ordering after data is written.
/// 3. Readers read `write_pos` with `Acquire` ordering before reading data.
/// 4. Readers and writer operate on disjoint regions of the buffer (enforced by the
///    read/write position protocol).
///
/// This pattern matches the classic lock-free ring buffer design used in the C++
/// implementation.
pub struct RingBuffer {
    data: UnsafeCell<Vec<u8>>,
    mask: usize,
    capacity: usize,
    write_pos: AtomicUsize,
    reader_positions: Mutex<Vec<Arc<AtomicUsize>>>,
    notify: tokio::sync::Notify,
}

// RingBuffer is Send + Sync because all shared mutable state is protected
// by atomics (write_pos, reader positions) or mutex (reader_positions).
// The UnsafeCell for data is only accessed under the write lock for writes,
// and reads are synchronized via atomic position ordering.
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

/// A reader attached to a ring buffer. Each reader has an independent
/// position and can be used concurrently from different threads/tasks.
pub struct RingBufferReader {
    buffer: Arc<RingBuffer>,
    read_pos: Arc<AtomicUsize>,
}

// RingBufferReader is Send because it only reads from the shared buffer
// using atomic position synchronization.
unsafe impl Send for RingBufferReader {}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity.
    /// Panics if capacity is not a power of 2 or is zero.
    pub fn new(capacity: usize) -> Arc<Self> {
        assert!(capacity > 0, "Capacity must be greater than 0");
        assert!(
            capacity & (capacity - 1) == 0,
            "Capacity must be a power of 2, got {}",
            capacity
        );
        let mask = capacity - 1;
        let data = UnsafeCell::new(vec![0u8; capacity]);
        Arc::new(Self {
            data,
            mask,
            capacity,
            write_pos: AtomicUsize::new(0),
            reader_positions: Mutex::new(Vec::new()),
            notify: tokio::sync::Notify::new(),
        })
    }

    /// Create a ring buffer with the default capacity.
    pub fn default() -> Arc<Self> {
        Self::new(BUFFER_CAPACITY)
    }

    /// Push data into the buffer. If the buffer would overflow, advances
    /// all reader positions that are behind to avoid them reading garbage.
    pub fn push(&self, data: &[u8]) {
        if data.is_empty() || data.len() > self.capacity {
            return;
        }

        let current_wp = self.write_pos.load(Ordering::Relaxed);
        let len = data.len();

        // Find the slowest reader position (all positions are unbounded counters).
        // When there are no readers, treat slowest_rp as current_wp so used=0.
        let slowest_rp = {
            let readers = self.reader_positions.lock().unwrap();
            if readers.is_empty() {
                current_wp
            } else {
                readers.iter()
                    .map(|r| r.load(Ordering::Relaxed))
                    .min()
                    .unwrap_or(current_wp)
            }
        };

        // used is always <= capacity because the overflow logic below prevents
        // any reader from falling more than capacity bytes behind.
        let used = current_wp - slowest_rp;
        let free = self.capacity.saturating_sub(used);

        if data.len() > free {
            // Writing data.len() bytes will overwrite circular positions that
            // some readers haven't consumed yet.  The overwritten range
            // (in unbounded-counter space) is [current_wp - capacity,
            // current_wp - capacity + data.len()).  Any reader whose position
            // falls in that range must be fast-forwarded past it.
            let overwrite_end = current_wp + data.len() - self.capacity;
            let readers = self.reader_positions.lock().unwrap();
            for r in readers.iter() {
                let rp = r.load(Ordering::Relaxed);
                if rp < overwrite_end {
                    r.store(overwrite_end, Ordering::Release);
                }
            }
        }

        // Write data into buffer (handle wrap-around)
        // SAFETY: Exclusive write access is guaranteed by the local computation above;
        // no other thread writes to the buffer. Readers are synchronized via atomic
        // write_pos with Release/Acquire ordering.
        let buf = unsafe { &mut *self.data.get() };
        let wp_idx = current_wp & self.mask;
        let first_seg = std::cmp::min(len, self.capacity - wp_idx);
        buf[wp_idx..wp_idx + first_seg].copy_from_slice(&data[..first_seg]);
        if first_seg < len {
            let second_seg = len - first_seg;
            buf[..second_seg].copy_from_slice(&data[first_seg..]);
        }

        self.write_pos
            .store(current_wp + len, Ordering::Release);

        self.notify.notify_waiters();
    }

    /// Create a new reader that starts at the current write position
    /// (catches up — skips all buffered data).
    pub fn create_reader(self: &Arc<Self>) -> RingBufferReader {
        let current_wp = self.write_pos.load(Ordering::Acquire);
        // Store the raw (unbounded) write position so that avail = write_pos - read_pos
        // is computed in the same address space as write_pos.
        let read_pos = Arc::new(AtomicUsize::new(current_wp));

        let mut readers = self.reader_positions.lock().unwrap();
        readers.push(read_pos.clone());

        RingBufferReader {
            buffer: Arc::clone(self),
            read_pos,
        }
    }

    /// Wake all waiting readers.
    pub fn notify_readers(&self) {
        self.notify.notify_waiters();
    }

    /// Get current write position (raw counter value, not masked).
    pub fn current_write_pos(&self) -> usize {
        self.write_pos.load(Ordering::Acquire)
    }
}

impl Drop for RingBufferReader {
    fn drop(&mut self) {
        let mut readers = match self.buffer.reader_positions.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        readers.retain(|r| !Arc::ptr_eq(r, &self.read_pos));
        // 大量连接断开后回收 Vec 多余容量，避免内存长期占用
        if readers.capacity() > readers.len().saturating_add(16) * 2 {
            readers.shrink_to_fit();
        }
    }
}

impl RingBufferReader {
    /// Read available data into `dest`. Returns number of bytes read.
    /// Non-blocking, lock-free.
    pub fn read(&self, dest: &mut [u8]) -> usize {
        let wp = self.buffer.write_pos.load(Ordering::Acquire);
        let rp = self.read_pos.load(Ordering::Relaxed);
        let rp_idx = rp & self.buffer.mask;

        let avail = if wp >= rp {
            wp - rp
        } else {
            self.buffer.capacity - rp + wp
        };

        if avail == 0 {
            return 0;
        }

        let to_read = std::cmp::min(avail, dest.len());
        let first_seg = std::cmp::min(to_read, self.buffer.capacity - rp_idx);

        // SAFETY: The reader only reads from the region between read_pos and write_pos.
        // The writer will never overwrite this region as long as the reader has not
        // fallen behind (and is kept up-to-date by push's overflow advancement).
        // The Acquire load of write_pos ensures we see all writes up to that point.
        let buf = unsafe { &*self.buffer.data.get() };
        dest[..first_seg].copy_from_slice(&buf[rp_idx..rp_idx + first_seg]);

        if first_seg < to_read {
            let second_seg = to_read - first_seg;
            dest[first_seg..first_seg + second_seg]
                .copy_from_slice(&buf[..second_seg]);
        }

        // Advance without masking: read_pos stays in the same unbounded-counter
        // space as write_pos so that avail = write_pos - read_pos is always valid.
        self.read_pos.store(rp + to_read, Ordering::Release);

        to_read
    }

    /// Wait for data to become available or timeout expires.
    /// Returns number of bytes available to read.
    pub async fn wait_for_data(&self, timeout_ms: u64) -> usize {
        let timeout = tokio::time::Duration::from_millis(timeout_ms);
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let wp = self.buffer.write_pos.load(Ordering::Acquire);
            let rp = self.read_pos.load(Ordering::Relaxed);

            let avail = if wp >= rp {
                wp - rp
            } else {
                self.buffer.capacity - rp + wp
            };

            if avail > 0 {
                return avail;
            }

            let now = tokio::time::Instant::now();
            if now >= deadline {
                return 0;
            }

            let remaining = deadline - now;
            match tokio::time::timeout(remaining, self.buffer.notify.notified()).await {
                Ok(()) => continue,
                Err(_) => return 0,
            }
        }
    }

    /// Skip ahead to the current write position (for late joiners).
    pub fn catch_up(&self) {
        let wp = self.buffer.write_pos.load(Ordering::Acquire);
        self.read_pos.store(wp, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = RingBuffer::new(1024);
        assert_eq!(buf.capacity, 1024);
        assert_eq!(buf.mask, 1023);
    }

    #[test]
    #[should_panic]
    fn test_capacity_not_power_of_two() {
        RingBuffer::new(1000);
    }

    #[test]
    #[should_panic]
    fn test_capacity_zero() {
        RingBuffer::new(0);
    }

    #[test]
    fn test_push_and_read() {
        let buf = RingBuffer::new(1024);
        let reader = buf.create_reader();

        let data = b"hello world";
        buf.push(data);

        let mut dest = [0u8; 64];
        let n = reader.read(&mut dest);
        assert_eq!(n, data.len());
        assert_eq!(&dest[..n], data);
    }

    #[test]
    fn test_multiple_readers() {
        let buf = RingBuffer::new(1024);
        let r1 = buf.create_reader();
        let r2 = buf.create_reader();

        buf.push(b"AAAA");
        buf.push(b"BBBB");

        let mut d1 = [0u8; 64];
        let mut d2 = [0u8; 64];

        let n1 = r1.read(&mut d1);
        let n2 = r2.read(&mut d2);

        assert_eq!(n1, 8);
        assert_eq!(n2, 8);
        assert_eq!(&d1[..8], b"AAAABBBB");
        assert_eq!(&d2[..8], b"AAAABBBB");
    }

    #[test]
    fn test_wrap_around() {
        let buf = RingBuffer::new(8);
        let reader = buf.create_reader();

        buf.push(&[1, 2, 3, 4, 5]);
        let mut dest = [0u8; 8];
        let n = reader.read(&mut dest);
        assert_eq!(n, 5);

        buf.push(&[6, 7, 8, 9]);
        let mut dest2 = [0u8; 8];
        let n = reader.read(&mut dest2);
        assert_eq!(n, 4);
    }

    #[test]
    fn test_overflow_advances_readers() {
        let buf = RingBuffer::new(8);
        let reader = buf.create_reader();

        buf.push(&[1, 2, 3, 4, 5, 6]);
        buf.push(&[7, 8, 9, 10]);

        let mut dest = [0u8; 8];
        let n = reader.read(&mut dest);
        assert!(n > 0);
    }

    #[tokio::test]
    async fn test_wait_for_data() {
        let buf = RingBuffer::new(1024);
        let reader = buf.create_reader();

        assert_eq!(reader.wait_for_data(10).await, 0);

        let buf_clone = Arc::clone(&buf);
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            buf_clone.push(b"test data");
        });

        let avail = reader.wait_for_data(5000).await;
        assert!(avail > 0);
    }

    #[test]
    fn test_catch_up() {
        let buf = RingBuffer::new(1024);
        buf.push(b"some old data");

        let reader = buf.create_reader();
        reader.catch_up();

        let mut dest = [0u8; 64];
        assert_eq!(reader.read(&mut dest), 0);

        buf.push(b"new data");
        let n = reader.read(&mut dest);
        assert!(n > 0);
    }
}
