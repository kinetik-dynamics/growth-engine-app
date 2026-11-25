import type { QueueItem } from "./api";

/** "13h ago" style age, the same wording the web app uses. */
export function timeAgo(isoDate: string): string {
  const seconds = Math.floor((Date.now() - new Date(isoDate).getTime()) / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export function externalUrl(value: string): string {
  return /^https?:\/\//i.test(value) ? value : `https://${value}`;
}

export function ratingLabel(item: Pick<QueueItem, "rating" | "reviews_count">): string | null {
  if (item.rating == null) return null;
  return `★ ${item.rating}${item.reviews_count ? ` (${item.reviews_count} reviews)` : ""}`;
}

export function primaryLabel(item: QueueItem): string {
  if (item.type === "lead") return item.contact_name || item.company_name || "?";
  return item.company_name || "?";
}

export function secondaryLabel(item: QueueItem): string | null {
  if (item.type === "lead" && item.maps_url) return [item.category, ratingLabel(item)].filter(Boolean).join(" · ") || null;
  if (item.type === "lead") return [item.contact_title, item.company_name].filter(Boolean).join(" at ") || null;
  return item.company_domain;
}
