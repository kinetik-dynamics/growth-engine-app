import { invoke } from "@tauri-apps/api/core";

export interface Session {
  baseUrl: string;
  userName: string;
  userEmail: string;
  companyName: string | null;
  websiteUrl: string | null;
}

export interface Offer {
  id: number;
  name: string;
  promise: string;
  description: string | null;
  price_range: string | null;
  timeline: string | null;
  deliverables: string[];
  drafted_by_agent: boolean;
  case_studies_count: number;
  leads_count: number;
  icps: { id: number; name: string; pain: string | null }[];
  case_studies: { id: number; title: string; client_name: string | null; result: string }[];
}

export type OfferPricing = "all" | "priced" | "unpriced";
export type OfferSource = "all" | "agent" | "manual";
export type OfferSort = "newest" | "drafted" | "name" | "proof" | "recent";

export interface OfferFilters {
  search?: string;
  icpId?: number | null;
  pricing?: OfferPricing;
  source?: OfferSource;
  sort?: OfferSort;
}

/** One page of offers from the server, plus totals that ignore the filters. */
export interface OfferPage {
  offers: Offer[];
  meta: { current_page: number; last_page: number; per_page: number; total: number };
  stats: { total: number; drafted_by_agent: number; priced: number; with_proof: number };
  icp_options: { id: number; name: string }[];
}

export type IdeaTab = "drafts" | "sent";

/** One page of the local ideas, latest first, plus the totals of both tabs. */
export interface IdeaPage {
  ideas: Idea[];
  page: number;
  lastPage: number;
  /** Ideas in the requested tab, across all pages. */
  total: number;
  draftCount: number;
  sentCount: number;
}

export interface CliStatus {
  id: string;
  name: string;
  command: string;
  installed: boolean;
  path: string | null;
  version: string | null;
}

export interface Idea {
  id: string;
  title: string;
  shortNotes: string;
  painAddressed: string;
  targetKeyword: string;
  pattern: string;
  tag: string;
  source: string;
  channelId: string | null;
  channelName: string | null;
  scheduledDate: string | null;
  scheduledTime: string | null;
  offerName: string | null;
  suggestedDraft: string;
  imagePrompt: string | null;
  status: string;
  provider: string | null;
  createdAt: number;
  /** When the idea was last sent to the main site (ms since epoch), if ever. */
  sentAt: number | null;
  remoteId: number | null;
  /** The channels picked on the Write post page, each with its schedule. */
  publishChannels: ChannelSchedule[];
  /** When the idea's post last joined the main site's publish queue (ms since epoch). */
  publishedAt: number | null;
}

export interface ChannelSchedule {
  id: string;
  scheduledDate: string | null;
  scheduledTime: string | null;
}

/** The fields the idea page can change. */
export type IdeaEdit = Pick<
  Idea,
  | "id"
  | "title"
  | "shortNotes"
  | "painAddressed"
  | "targetKeyword"
  | "pattern"
  | "tag"
  | "channelId"
  | "channelName"
  | "suggestedDraft"
  | "imagePrompt"
  | "publishChannels"
>;

/** A connected account on the main site a post can go out on. */
export interface ComposerChannel {
  id: string;
  platform: string;
  name: string;
  handleOrSub: string;
  avatarUrl: string | null;
}

/** The main site's copy of a sent idea, as its Write post page shows it. */
export interface RemoteComposer {
  idea: {
    id: number;
    title: string;
    shortNotes: string | null;
    painAddressed: string | null;
    targetKeyword: string | null;
    pattern: string | null;
    tag: string | null;
    source: string;
    provider: string | null;
    offerName: string | null;
    plannedDate: string | null;
    plannedTime: string | null;
    suggestedDraft: string | null;
    imagePrompt: string | null;
    publishStatus: "Draft" | "Sent" | null;
  };
  channels: ComposerChannel[];
  posts: { channelId: string; text: string; pattern: string | null; status: string; scheduledDate: string | null; scheduledTime: string | null }[];
}

export interface BulkIdeaSchedule {
  id: string;
  scheduledDate: string;
  scheduledTime: string;
}

export interface PlanSlot {
  channelId: string;
  channelName: string;
  platform: string;
}

export interface PlanRequest {
  provider: string;
  offerId: number;
  slots: PlanSlot[];
  withImages: boolean;
}

export interface OfferIdeaCount {
  offerId: number | null;
  offerName: string | null;
  count: number;
}

export interface Permission {
  provider: string;
  tool: string;
}

export type PlanOutcome =
  | { kind: "created"; ideas: Idea[] }
  | { kind: "permissionNeeded"; provider: string; tools: string[] };

/** One line of the live activity feed emitted while a plan generates. */
export interface PlanProgress {
  kind: "stage" | "text" | "tool" | "denied" | "log";
  message: string;
}

export interface Named {
  id: number;
  name: string;
}

export interface Overview {
  profile: { company_name: string | null; website_url: string | null; logo_url: string | null };
  stats: {
    leads: { total: number; thisWeek: number; lastWeek: number };
    companies: { total: number; thisWeek: number };
    emails: { found: number; missing: number };
    posts: { sent30d: number; sentPrev30d: number; scheduled: number; drafts: number; failed: number };
    ideas: { total: number; unused: number };
  };
  trend: { date: string; leads: number; posts: number }[];
  pipeline: { id: number | null; name: string; color: string; count: number }[];
  icps: { id: number; name: string; count: number }[];
  recentLeads: {
    key: string;
    type: "lead" | "prospect";
    name: string;
    subtitle: string | null;
    avatarUrl: string | null;
    domain: string | null;
    hasEmail: boolean;
    createdAt: string;
  }[];
  upcomingPosts: { id: number; channel: string; platform: string | null; text: string; scheduledAt: string | null }[];
  channels: { id: string; platform: string; name: string; handle: string; avatarUrl: string | null }[];
  brain: { offers: number; icps: number; proof: number };
}

export interface SiteIcp {
  id: number;
  name: string;
  who_they_are: string;
  roles_to_contact: string | null;
  buying_triggers: string | null;
  pain: string | null;
  criteria: string[] | null;
  offers: Named[];
  drafted_by_agent: boolean;
  leads_count: number;
  prospects_count: number;
  searches_count: number;
  updated_at: string;
}

export interface SiteProof {
  id: number;
  title: string;
  client_name: string | null;
  client_type: string | null;
  industry: string | null;
  result: string;
  can_name_client: boolean;
  drafted_by_agent: boolean;
  offer_id: number | null;
  offer: Named | null;
  updated_at: string;
}

export interface PublishChannel {
  id: string;
  platform: string;
  name: string;
  handleOrSub: string;
  avatarUrl: string | null;
  status: string;
  isPaused: boolean;
}

export type PostStatus = "Draft" | "Needs approval" | "Scheduled" | "Publishing" | "Sent" | "Failed";

export interface SitePost {
  id: string;
  ideaId: number | null;
  channelId: string;
  text: string;
  status: PostStatus;
  scheduledDate: string | null;
  scheduledTime: string | null;
  sentAt: string | null;
  errorMessage: string | null;
  siblingPostIds: string[];
  tag: string | null;
  pattern: string | null;
}

export interface SitePages {
  overview: { overview: Overview | null };
  icps: { icps: SiteIcp[]; offerOptions: Named[] };
  proof: { proof: SiteProof[]; offerOptions: Named[] };
  publish: { channels: PublishChannel[]; posts: SitePost[] };
}

/** Ideas generated from an offer. Older ideas only recorded the offer's name. */
export function ideaCountFor(offer: Offer, counts: OfferIdeaCount[]): number {
  return counts
    .filter((c) => c.offerId === offer.id || (c.offerId === null && c.offerName === offer.name))
    .reduce((sum, c) => sum + c.count, 0);
}

export const api = {
  offerIdeaCounts: () => invoke<OfferIdeaCount[]>("offer_idea_counts"),
  listPermissions: () => invoke<Permission[]>("list_permissions"),
  grantPermissions: (provider: string, tools: string[]) => invoke<void>("grant_permissions", { provider, tools }),
  revokePermission: (provider: string, tool: string) => invoke<void>("revoke_permission", { provider, tool }),
  updateIdea: (idea: IdeaEdit) => invoke<Idea>("update_idea", { idea }),
  sendIdea: (id: string) => invoke<Idea>("send_idea", { id }),
  publishIdea: (id: string) => invoke<Idea>("publish_idea", { id }),
  publishIdeas: (ideas: BulkIdeaSchedule[], channels: string[]) => invoke<Idea[]>("publish_ideas", { ideas, channels }),
  listChannels: () => invoke<{ channels: ComposerChannel[] }>("list_channels"),
  getRemoteIdea: (remoteId: number) => invoke<RemoteComposer>("get_remote_idea", { remoteId }),
  listPatterns: () => invoke<string[]>("list_patterns"),
  cancelPlan: () => invoke<void>("cancel_plan"),
  getSession: () => invoke<Session | null>("get_session"),
  login: (baseUrl: string, token: string) => invoke<Session>("login", { baseUrl, token }),
  logout: () => invoke<void>("logout"),
  listOffers: (page = 1, filters: OfferFilters = {}) =>
    invoke<OfferPage>("list_offers", {
      page,
      search: filters.search ?? "",
      icpId: filters.icpId ?? null,
      pricing: filters.pricing ?? null,
      source: filters.source ?? null,
      sort: filters.sort ?? null,
    }),
  sitePage: <K extends keyof SitePages>(page: K) => invoke<SitePages[K]>("site_page", { page }),
  detectClis: () => invoke<CliStatus[]>("detect_clis"),
  testCli: (provider: string) => invoke<string>("test_cli", { provider }),
  getSkill: () => invoke<string>("get_skill"),
  saveSkill: (content: string) => invoke<void>("save_skill", { content }),
  resetSkill: () => invoke<string>("reset_skill"),
  listIdeas: (tab: IdeaTab, page = 1) => invoke<IdeaPage>("list_ideas", { tab, page }),
  addIdea: (title: string) => invoke<Idea>("add_idea", { title }),
  deleteIdea: (id: string) => invoke<void>("delete_idea", { id }),
  generatePlan: (request: PlanRequest) => invoke<PlanOutcome>("generate_plan", { request }),
};

/** Tauri commands reject with a plain string; anything else is unexpected. */
export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}
