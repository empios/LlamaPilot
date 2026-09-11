// Keep the page fully usable without JavaScript. Add a small, accurate OS hint only.
const platform = navigator.userAgentData?.platform || navigator.platform || "";
const mobile =
  navigator.userAgentData?.mobile ||
  /Android|iPhone|iPad|iPod/i.test(navigator.userAgent) ||
  (/Mac/i.test(platform) && navigator.maxTouchPoints > 1);
const family = mobile
  ? null
  : /Mac/i.test(platform)
    ? "macos"
    : /Win/i.test(platform)
      ? "windows"
      : /Linux/i.test(platform)
        ? "linux"
        : null;
if (family) {
  const label = document.querySelector(
    `#download-${family} .platform-heading .mono`,
  );
  if (label) label.textContent += " · YOUR OS";
}
