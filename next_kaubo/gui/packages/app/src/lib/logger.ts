/** Minimal debug logger — only active in dev mode. No npm deps. */
const isDev = import.meta.env.DEV;

export const log = {
  lex(label: string, sourceLen: number) {
    if (!isDev) return;
    console.debug(`[kaubo:lex:${label}] source=${sourceLen}B`);
  },
  token(label: string, count: number, sample?: unknown) {
    if (!isDev) return;
    console.debug(`[kaubo:token:${label}] ${count} tokens`, sample ?? "");
  },
  deco(label: string, count: number) {
    if (!isDev) return;
    console.debug(`[kaubo:deco:${label}] ${count} marks`);
  },
  err(label: string, msg: string) {
    console.error(`[kaubo:${label}] ${msg}`);
  },
};
