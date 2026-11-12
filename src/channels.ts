import type { PlanSlot } from "./api";

export interface Channel {
  id: string;
  platform: "LinkedIn" | "Facebook" | "X" | "Website";
  name: string;
  color: string;
  defaultCount: number;
}

export const MAX_POSTS_PER_CHANNEL = 7;

/** Mirrors the web app's channels and their weekly posting goals. */
export function channelsFor(websiteUrl: string | null): Channel[] {
  const domain = websiteUrl ? websiteUrl.replace(/^https?:\/\//, "").replace(/\/.*$/, "") : "blog";
  return [
    { id: "linkedin_personal", platform: "LinkedIn", name: "LinkedIn · Personal profile", color: "#0a66c2", defaultCount: 3 },
    { id: "facebook_page", platform: "Facebook", name: "Facebook · Page", color: "#1877f2", defaultCount: 1 },
    { id: "x_profile", platform: "X", name: "X · account", color: "#111827", defaultCount: 5 },
    { id: "website_blog", platform: "Website", name: `Website · ${domain}`, color: "#047857", defaultCount: 1 },
  ];
}

/** Monday of the week after `today`. */
export function nextMonday(today = new Date()): Date {
  const weekday = today.getDay() === 0 ? 7 : today.getDay();
  const monday = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  monday.setDate(monday.getDate() + (8 - weekday));
  return monday;
}

export function weekLabel(monday: Date): string {
  const sunday = new Date(monday);
  sunday.setDate(sunday.getDate() + 6);
  const format = (date: Date) => date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  return `${format(monday)} – ${format(sunday)}, ${sunday.getFullYear()}`;
}

/** One idea request per post the user asked for on each channel. */
export function slotsFor(channels: Channel[], counts: Record<string, number>): PlanSlot[] {
  return channels.flatMap((channel) =>
    Array.from({ length: counts[channel.id] ?? 0 }, () => ({
      channelId: channel.id,
      channelName: channel.name,
      platform: channel.platform,
    })),
  );
}
