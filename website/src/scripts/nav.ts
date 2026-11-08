// The bar gets its background once the page has moved.
const nav = document.querySelector<HTMLElement>('[data-nav]');
if (nav) {
  const update = () => nav.toggleAttribute('data-scrolled', window.scrollY > 16);
  update();
  window.addEventListener('scroll', update, { passive: true });
}
