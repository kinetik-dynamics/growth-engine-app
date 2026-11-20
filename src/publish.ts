import type { PublishChannel, SitePost } from "./api";

export const CONNECTIONS_PATH = "/settings/connections_destinations";

/** The web app's Publish tab that holds this post and its actions. */
export function publishTabPath(post: SitePost) {
  const tab = { Scheduled: "queue", Draft: "drafts", "Needs approval": "approvals" }[post.status as string] ?? "sent";
  return `/publish?tab=${tab}`;
}

/** Posts are written and edited on their idea's Write post page. */
export function editPath(post: SitePost) {
  return post.ideaId ? `/ideas/${post.ideaId}/write` : publishTabPath(post);
}

export const DAY_NAMES = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const;
const FULL_DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Today in this computer's timezone, as "YYYY-MM-DD". Schedules are wall-clock times. */
export function todayLocalDate() {
  const date = new Date();
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function parseDateStr(dateStr: string) {
  const [y, m, d] = dateStr.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d));
}

export function addDays(dateStr: string, days: number) {
  const date = parseDateStr(dateStr);
  date.setUTCDate(date.getUTCDate() + days);
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}-${String(date.getUTCDate()).padStart(2, "0")}`;
}

export function dayOfWeek(dateStr: string) {
  return DAY_NAMES[parseDateStr(dateStr).getUTCDay()];
}

/** "Today", "Tomorrow" or "Thursday, Oct 29". */
export function friendlyDate(dateStr: string, today: string) {
  if (dateStr === today) return "Today";
  if (dateStr === addDays(today, 1)) return "Tomorrow";
  const date = parseDateStr(dateStr);
  return `${FULL_DAY_NAMES[date.getUTCDay()]}, ${MONTHS[date.getUTCMonth()]} ${date.getUTCDate()}`;
}

/** The short label the web app's channel panel shows for each channel. */
export function channelLabel(channel: PublishChannel) {
  if (channel.platform === "LinkedIn") return channel.id === "linkedin_personal" ? "LinkedIn · Personal" : "LinkedIn · Company";
  if (channel.platform === "Facebook") return "Facebook · Page";
  if (channel.platform === "X") return "X · Account";
  return "Website";
}
