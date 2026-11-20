/**
 * Text formatting for social posts and blog CMS, ported from the web app:
 * Unicode styles (render natively on LinkedIn, X and Facebook), Markdown
 * (for website/CMS), lists, quotes, case changes and clearing it all.
 */

export type FormatMode = "unicode" | "markdown";

export type FormatAction =
  | "bold"
  | "italic"
  | "underline"
  | "strikethrough"
  | "mono"
  | "bullets"
  | "numbers"
  | "quote"
  | "uppercase"
  | "titlecase"
  | "clear";

const BOLD_CAPS_OFFSET = 120211;
const BOLD_LOW_OFFSET = 120205;
const BOLD_DIGIT_OFFSET = 120764;
const ITALIC_CAPS_OFFSET = 120263;
const ITALIC_LOW_OFFSET = 120257;
const MONO_CAPS_OFFSET = 120367;
const MONO_LOW_OFFSET = 120361;
const MONO_DIGIT_OFFSET = 120774;

function mapLetters(text: string, caps: number, low: number, digit?: number): string {
  let result = "";
  for (const char of text) {
    const code = char.charCodeAt(0);
    if (code >= 65 && code <= 90) result += String.fromCodePoint(code + caps);
    else if (code >= 97 && code <= 122) result += String.fromCodePoint(code + low);
    else if (digit !== undefined && code >= 48 && code <= 57) result += String.fromCodePoint(code + digit);
    else result += char;
  }
  return result;
}

export const toUnicodeBold = (text: string) => mapLetters(text, BOLD_CAPS_OFFSET, BOLD_LOW_OFFSET, BOLD_DIGIT_OFFSET);
export const toUnicodeItalic = (text: string) => mapLetters(text, ITALIC_CAPS_OFFSET, ITALIC_LOW_OFFSET);
export const toUnicodeMonospace = (text: string) => mapLetters(text, MONO_CAPS_OFFSET, MONO_LOW_OFFSET, MONO_DIGIT_OFFSET);

/**
 * Adds a combining mark after each character. Splits by code point, so
 * emoji and already-bold letters (two UTF-16 units each) stay whole.
 */
function withMark(text: string, mark: string): string {
  return Array.from(text).map((c) => (c === " " || c === "\n" || c === mark ? c : c + mark)).join("");
}

export const toUnicodeStrikethrough = (text: string) => withMark(text, "̶");
export const toUnicodeUnderline = (text: string) => withMark(text, "̲");

/** Strips Unicode styles, combining marks and Markdown back to plain text. */
export function clearFormatting(text: string): string {
  let clean = text.replace(/[̶̲]/g, "");
  clean = clean.replace(/\*\*(.*?)\*\*/g, "$1");
  clean = clean.replace(/\*(.*?)\*/g, "$1");
  clean = clean.replace(/~~(.*?)~~/g, "$1");
  clean = clean.replace(/`([^`]+)`/g, "$1");

  let output = "";
  for (const char of clean) {
    const cp = char.codePointAt(0) || 0;
    if (cp >= 0x1d5d4 && cp <= 0x1d5ed) output += String.fromCharCode(cp - BOLD_CAPS_OFFSET);
    else if (cp >= 0x1d5ee && cp <= 0x1d607) output += String.fromCharCode(cp - BOLD_LOW_OFFSET);
    else if (cp >= 0x1d7ec && cp <= 0x1d7f5) output += String.fromCharCode(cp - BOLD_DIGIT_OFFSET);
    else if (cp >= 0x1d400 && cp <= 0x1d419) output += String.fromCharCode(cp - 0x1d400 + 65);
    else if (cp >= 0x1d41a && cp <= 0x1d433) output += String.fromCharCode(cp - 0x1d41a + 97);
    else if (cp >= 0x1d7ce && cp <= 0x1d7d7) output += String.fromCharCode(cp - 0x1d7ce + 48);
    else if (cp >= 0x1d608 && cp <= 0x1d621) output += String.fromCharCode(cp - ITALIC_CAPS_OFFSET);
    else if (cp >= 0x1d622 && cp <= 0x1d63b) output += String.fromCharCode(cp - ITALIC_LOW_OFFSET);
    else if (cp >= 0x1d434 && cp <= 0x1d44d) output += String.fromCharCode(cp - 0x1d434 + 65);
    else if (cp >= 0x1d44e && cp <= 0x1d467) output += String.fromCharCode(cp - 0x1d44e + 97);
    else if (cp >= 0x1d670 && cp <= 0x1d689) output += String.fromCharCode(cp - MONO_CAPS_OFFSET);
    else if (cp >= 0x1d68a && cp <= 0x1d6a3) output += String.fromCharCode(cp - MONO_LOW_OFFSET);
    else if (cp >= 0x1d7f6 && cp <= 0x1d800) output += String.fromCharCode(cp - MONO_DIGIT_OFFSET);
    else output += char;
  }
  return output;
}

export function toTitleCase(text: string): string {
  return text.replace(/\b\w+/g, (word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase());
}

export function toBulletList(text: string): string {
  return text
    .split("\n")
    .map((line) => {
      const trimmed = line.trim();
      if (!trimmed) return line;
      if (trimmed.startsWith("• ") || trimmed.startsWith("- ") || trimmed.startsWith("* ")) return trimmed;
      return `• ${trimmed}`;
    })
    .join("\n");
}

export function toNumberedList(text: string): string {
  let count = 1;
  return text
    .split("\n")
    .map((line) => {
      const trimmed = line.trim();
      if (!trimmed) return line;
      return `${count++}. ${trimmed.replace(/^\d+[.)]\s*/, "")}`;
    })
    .join("\n");
}

export function toQuote(text: string): string {
  return text
    .split("\n")
    .map((line) => {
      if (!line.trim()) return line;
      if (line.trim().startsWith("> ")) return line;
      return `> ${line.trim()}`;
    })
    .join("\n");
}

/** Formats the selection (or a placeholder) and returns the new text and selection. */
export function applyFormatToTextarea(
  fullText: string,
  selectionStart: number,
  selectionEnd: number,
  action: FormatAction,
  mode: FormatMode = "unicode",
): { newText: string; newStart: number; newEnd: number } {
  const hasSelection = selectionStart !== selectionEnd;
  const selected = hasSelection ? fullText.substring(selectionStart, selectionEnd) : "";

  if (action === "clear" && !hasSelection) {
    return { newText: clearFormatting(fullText), newStart: selectionStart, newEnd: selectionStart };
  }

  const shared: Partial<Record<FormatAction, () => string>> = {
    bullets: () => toBulletList(selected || "First key point\nSecond key point\nThird key point"),
    numbers: () => toNumberedList(selected || "First priority\nSecond priority\nThird priority"),
    uppercase: () => (selected || fullText).toUpperCase(),
    titlecase: () => toTitleCase(selected || fullText),
    clear: () => clearFormatting(selected || fullText),
  };
  const markdown: Partial<Record<FormatAction, () => string>> = {
    bold: () => (hasSelection ? `**${selected}**` : "**bold text**"),
    italic: () => (hasSelection ? `*${selected}*` : "*italic text*"),
    strikethrough: () => (hasSelection ? `~~${selected}~~` : "~~strikethrough~~"),
    mono: () => (hasSelection ? `\`${selected}\`` : "`code`"),
    quote: () => toQuote(selected || "High-signal quote or key insight"),
  };
  const unicode: Partial<Record<FormatAction, () => string>> = {
    bold: () => toUnicodeBold(selected || "Bold Headline"),
    italic: () => toUnicodeItalic(selected || "Italic emphasis"),
    underline: () => toUnicodeUnderline(selected || "Underlined hook"),
    strikethrough: () => toUnicodeStrikethrough(selected || "Outdated method"),
    mono: () => toUnicodeMonospace(selected || "TechBenchmark.v2"),
    quote: () => toQuote(selected || "High-signal client quote or takeaway"),
  };

  const transform = (mode === "markdown" ? markdown : unicode)[action] ?? shared[action];
  const transformed = transform ? transform() : selected;

  const newText = fullText.substring(0, selectionStart) + transformed + fullText.substring(selectionEnd);
  return { newText, newStart: selectionStart, newEnd: selectionStart + transformed.length };
}

/** Markdown to HTML for the live preview; the text is escaped first. Unicode styles pass through. */
export function formatPostPreviewHtml(text: string): string {
  if (!text) return "";
  let safe = text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  safe = safe.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>");
  safe = safe.replace(/(?<!\*)\*(?!\*)(.*?)(?<!\*)\*(?!\*)/g, "<em>$1</em>");
  safe = safe.replace(/~~(.*?)~~/g, '<del class="wp-del">$1</del>');
  safe = safe.replace(/`([^`]+)`/g, '<code class="wp-code">$1</code>');
  return safe;
}
