#include "client_connection.hpp"
#include "broadcast_buffer.hpp"

bool ClientConnection::send_audio() {
    if (shutdown_) return false;
    char tmp[Config::AUDIO_CHUNK_SIZE];
    size_t read_bytes = buffer_->read(consume_pos_, tmp, sizeof(tmp));
    if (read_bytes == 0) return true;
    ssize_t sent = ::send(fd_, tmp, read_bytes, MSG_NOSIGNAL);
    return sent >= 0 || errno == EAGAIN || errno == EWOULDBLOCK;
}
