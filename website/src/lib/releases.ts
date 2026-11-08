// Reads the latest GitHub release at build time, so visitors never contact a third party
// from this page. The site rebuilds on every release and daily (.github/workflows/website.yml).
//
// Asset names: the current scheme is `OpenDJDecks-<v>-…`, the release published under the
// old name (v0.3.0) still carries `OpenDecks-Session-<v>-…`, and the deb/tar/pi packages use
// lower-case variants of either. Every pattern below matches both.

export type Platform = 'macos' | 'windows' | 'linux' | 'pi';
export type Kind = 'dmgArm' | 'dmgIntel' | 'zip' | 'debAmd' | 'debArm' | 'appimageX86' | 'appimageArm' | 'tarX86' | 'tarArm' | 'pi';

export interface Asset {
  kind: Kind;
  platform: Platform;
  name: string;
  url: string;
  sizeMb: number;
}

export interface Release {
  version: string;
  url: string;
  publishedAt: Date;
  assets: Asset[];
  checksumsUrl?: string;
}

const api = 'https://api.github.com/repos/nurkert/OpenDJDecks/releases/latest';

const name = String.raw`(?:OpenDJDecks|OpenDecks-Session|opendjdecks|opendecks-session)`;
const kinds: [Kind, Platform, RegExp][] = [
  ['dmgArm', 'macos', new RegExp(`^${name}-.*-macos-arm64\\.dmg$`, 'i')],
  ['dmgIntel', 'macos', new RegExp(`^${name}-.*-macos-x86_64\\.dmg$`, 'i')],
  ['zip', 'windows', new RegExp(`^${name}-.*-windows-x86_64\\.zip$`, 'i')],
  ['debAmd', 'linux', new RegExp(`^${name}_.*_amd64\\.deb$`, 'i')],
  ['debArm', 'linux', new RegExp(`^${name}_.*_arm64\\.deb$`, 'i')],
  ['appimageX86', 'linux', new RegExp(`^${name}-.*-x86_64\\.AppImage$`, 'i')],
  ['appimageArm', 'linux', new RegExp(`^${name}-.*-aarch64\\.AppImage$`, 'i')],
  ['tarX86', 'linux', new RegExp(`^${name}-.*-linux-x86_64\\.tar\\.gz$`, 'i')],
  ['tarArm', 'linux', new RegExp(`^${name}-.*-linux-aarch64\\.tar\\.gz$`, 'i')],
  ['pi', 'pi', /^(?:opendjdecks|opendecks)-pi-.*\.tar\.gz$/i],
];

interface ApiAsset {
  name: string;
  browser_download_url: string;
  size: number;
}
interface ApiRelease {
  tag_name: string;
  html_url: string;
  published_at: string;
  assets: ApiAsset[];
}

/** Sort and classify the assets of one release; unknown files are skipped. */
export function classify(assets: ApiAsset[]): Asset[] {
  return assets
    .flatMap((asset) => {
      const match = kinds.find(([, , pattern]) => pattern.test(asset.name));
      if (!match) return [];
      return [
        {
          kind: match[0],
          platform: match[1],
          name: asset.name,
          url: asset.browser_download_url,
          sizeMb: Math.round((asset.size / 1_048_576) * 10) / 10,
        },
      ];
    })
    .sort((a, b) => kinds.findIndex(([k]) => k === a.kind) - kinds.findIndex(([k]) => k === b.kind));
}

let cached: Promise<Release | null> | undefined;

/** The one latest release; `null` when GitHub cannot be reached at build time. */
export function loadRelease(): Promise<Release | null> {
  cached ??= fetchRelease();
  return cached;
}

async function fetchRelease(): Promise<Release | null> {
  const headers: Record<string, string> = { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  try {
    const response = await fetch(api, { headers, signal: AbortSignal.timeout(10_000) });
    if (!response.ok) return null;
    const data = (await response.json()) as ApiRelease;
    return {
      version: data.tag_name.replace(/^v/, ''),
      url: data.html_url,
      publishedAt: new Date(data.published_at),
      assets: classify(data.assets),
      checksumsUrl: data.assets.find((asset) => /^SHA256SUMS/i.test(asset.name))?.browser_download_url,
    };
  } catch {
    return null;
  }
}
