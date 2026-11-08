// What the visitor's browser reveals about the computer. Only used for a recommendation on
// the page; nothing leaves the browser.
export interface Detected {
  os: 'macos' | 'windows' | 'linux' | '';
  /** 'arm', 'x86' or '' when the browser does not say. */
  arch: 'arm' | 'x86' | '';
}

interface UaData {
  platform?: string;
  architecture?: string;
  getHighEntropyValues?: (hints: string[]) => Promise<{ architecture?: string; platform?: string }>;
}

export function detectOs(): Detected['os'] {
  const data = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  const platform = (data?.platform || navigator.platform || navigator.userAgent).toLowerCase();
  // Phones and tablets do not run the app: Android reports as Linux, an iPad in desktop mode as Mac.
  if (/android|iphone|ipad|ipod/i.test(navigator.userAgent)) return '';
  if (navigator.maxTouchPoints > 1 && platform.includes('mac')) return '';
  if (platform.includes('mac')) return 'macos';
  if (platform.includes('win')) return 'windows';
  if (platform.includes('linux') || platform.includes('x11')) return 'linux';
  return '';
}

/** Apple Silicon versus Intel, where the browser can tell; otherwise empty. */
export async function detectArch(): Promise<Detected['arch']> {
  const data = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  try {
    const hints = await data?.getHighEntropyValues?.(['architecture']);
    if (hints?.architecture === 'arm') return 'arm';
    if (hints?.architecture === 'x86') return 'x86';
  } catch {
    // Not allowed: fall through.
  }
  // Safari and Firefox report no architecture. A WebGL renderer naming Apple is a safe "arm";
  // anything else stays unknown and the page offers both Mac downloads.
  try {
    const gl = document.createElement('canvas').getContext('webgl');
    const info = gl?.getExtension('WEBGL_debug_renderer_info');
    const renderer = info ? String(gl!.getParameter(info.UNMASKED_RENDERER_WEBGL)) : '';
    if (/apple m\d|apple gpu/i.test(renderer)) return 'arm';
  } catch {
    // No WebGL: unknown.
  }
  return '';
}
