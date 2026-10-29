import { invoke } from "@tauri-apps/api/core";
import type { TutorialActionPayload, TutorialRenderRequest } from "./tutorial-types";
import { createTutorial, type TutorialController } from "./tutorial";

function tabActive(): boolean {
  return document.querySelector(".top-tab.active")?.getAttribute("data-tab") === "gphotos";
}

async function setPhotosVisible(visible: boolean): Promise<void> {
  await invoke("gphotos_set_visible", { visible: visible && tabActive() });
}

export function attachTutorial(onComplete: (dismissForever: boolean) => void): TutorialController {
  const root = document.getElementById("gphotos-tutorial") as HTMLElement;
  const spotlight = root.querySelector("#tutorial-spotlight") as HTMLElement;
  const card = root.querySelector("#tutorial-card") as HTMLElement;
  const textEl = root.querySelector("#tutorial-text") as HTMLElement;
  const stepLabel = root.querySelector("#tutorial-step") as HTMLElement;
  const dismissCheckbox = root.querySelector("#tutorial-dismiss") as HTMLInputElement;
  const btnSkip = root.querySelector("#tutorial-skip") as HTMLButtonElement;
  const btnNext = root.querySelector("#tutorial-next") as HTMLButtonElement;
  const actions = new Set<(action: TutorialActionPayload) => void>();
  const reposition = new Set<() => void>();

  function render(request: TutorialRenderRequest): Promise<void> {
    if (request.mode === "intro") dismissCheckbox.checked = false;
    textEl.innerHTML = request.html;
    stepLabel.textContent = request.stepLabel;
    btnSkip.textContent = request.skipLabel;
    btnNext.textContent = request.nextLabel;

    const host = document.getElementById("tab-gphotos");
    const hostRect = host?.getBoundingClientRect();
    const target = request.targetSelector
      ? document.querySelector(request.targetSelector)
      : null;
    if (target && hostRect) {
      const rect = target.getBoundingClientRect();
      spotlight.classList.remove("hidden");
      spotlight.style.top = `${rect.top - hostRect.top}px`;
      spotlight.style.left = `${rect.left - hostRect.left}px`;
      spotlight.style.width = `${rect.width}px`;
      spotlight.style.height = `${rect.height}px`;
      card.classList.remove("is-centered");
      card.style.top = `${rect.bottom - hostRect.top + 8}px`;
      card.style.left = `${rect.left - hostRect.left}px`;
      card.style.maxWidth = "280px";
    } else {
      spotlight.classList.add("hidden");
      card.classList.add("is-centered");
      card.style.top = "40%";
      card.style.left = "50%";
      card.style.maxWidth = "280px";
    }
    return Promise.resolve();
  }

  btnSkip.addEventListener("click", () => {
    for (const cb of actions) cb({ type: "skip", dismissForever: dismissCheckbox.checked });
  });
  btnNext.addEventListener("click", () => {
    for (const cb of actions) cb({ type: "next", dismissForever: dismissCheckbox.checked });
  });
  window.addEventListener("resize", () => {
    for (const cb of reposition) cb();
  });

  return createTutorial(
    {
      show: () => {
        root.classList.remove("hidden");
        void setPhotosVisible(false);
      },
      hide: () => {
        root.classList.add("hidden");
        void setPhotosVisible(true);
      },
      render,
      onAction: (cb) => {
        actions.add(cb);
        return () => actions.delete(cb);
      },
      onReposition: (cb) => {
        reposition.add(cb);
        return () => reposition.delete(cb);
      },
    },
    onComplete,
  );
}
