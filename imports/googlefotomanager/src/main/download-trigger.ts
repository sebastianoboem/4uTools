import type { WebContents } from 'electron'

const delay = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms))

const TRIGGER_DOWNLOAD_MENU_SCRIPT = `
(function() {
  const moreLabels = ['Altre opzioni', 'More options', 'Altre azioni', 'More actions'];
  let menuBtn = null;
  for (const label of moreLabels) {
    menuBtn = document.querySelector('button[aria-label="' + label + '"]');
    if (menuBtn) break;
  }
  if (!menuBtn) return { ok: false, step: 'menu-button' };
  menuBtn.click();
  return { ok: true, step: 'menu-opened' };
})()
`

const TRIGGER_DOWNLOAD_ITEM_SCRIPT = `
(function() {
  const labels = ['Scarica', 'Download'];
  const candidates = [
    ...document.querySelectorAll('[role="menuitem"]'),
    ...document.querySelectorAll('[role="option"]'),
    ...document.querySelectorAll('li')
  ];
  for (const label of labels) {
    const item = candidates.find((el) => {
      const text = (el.textContent || '').trim();
      const aria = el.getAttribute('aria-label') || '';
      return text === label || text.startsWith(label + ' ') || aria === label;
    });
    if (item) {
      item.click();
      return { ok: true, method: 'menu' };
    }
  }
  return { ok: false, step: 'menu-item' };
})()
`

export async function triggerPhotosDownload(webContents: WebContents): Promise<{
  ok: boolean
  method?: string
  step?: string
}> {
  webContents.focus()

  const menuResult = (await webContents.executeJavaScript(TRIGGER_DOWNLOAD_MENU_SCRIPT)) as {
    ok: boolean
    step?: string
  }

  if (menuResult.ok) {
    await delay(350)
    const itemResult = (await webContents.executeJavaScript(TRIGGER_DOWNLOAD_ITEM_SCRIPT)) as {
      ok: boolean
      method?: string
      step?: string
    }
    if (itemResult.ok) return { ok: true, method: 'menu' }
  }

  await sendShiftD(webContents)
  await delay(300)
  return { ok: true, method: 'shortcut' }
}

async function sendShiftD(webContents: WebContents): Promise<void> {
  const modifiers: ('shift')[] = ['shift']
  webContents.sendInputEvent({ type: 'keyDown', keyCode: 'D', modifiers })
  await delay(40)
  webContents.sendInputEvent({ type: 'keyUp', keyCode: 'D', modifiers })
}
