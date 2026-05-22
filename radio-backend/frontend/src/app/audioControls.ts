export function volumeDown(audioEl: HTMLAudioElement): void {
  audioEl.volume = Math.max(0, audioEl.volume - 0.1)
}

export function volumeUp(audioEl: HTMLAudioElement): void {
  audioEl.volume = Math.min(1, audioEl.volume + 0.1)
}
