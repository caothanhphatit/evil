export function node<K extends keyof HTMLElementTagNameMap>(tag: K, className?: string, text?: string): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

export function unavailable(message: string): HTMLElement {
  return node("p", "hunter-info-unavailable", message);
}

export function sourceImage(path: string, alt = ""): HTMLImageElement {
  const image = node("img");
  image.src = path;
  image.alt = alt;
  return image;
}
