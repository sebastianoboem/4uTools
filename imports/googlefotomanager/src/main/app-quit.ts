type QuitPreparer = () => void

let prepareQuit: QuitPreparer | null = null

export function setPrepareAppQuit(fn: QuitPreparer): void {
  prepareQuit = fn
}

export function prepareAppQuit(): void {
  prepareQuit?.()
}
