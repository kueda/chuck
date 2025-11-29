// https://stackoverflow.com/a/47140708
export function stripTags(html: string) {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  return doc.body.textContent || '';
}
