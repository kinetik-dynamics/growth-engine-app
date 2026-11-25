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

/** An offer the AI agent drafted, kept in the app until it is synced to the main site. */
export interface OfferDraft {
  id: string;
  name: string;
  promise: string;
  description: string;
  priceRange: string;
  timeline: string;
  deliverables: string[];
  /** Ids of the main site's ICPs this offer is for. */
  icpIds: number[];
  /** Ids of the main site's case studies that prove it. */
  caseStudyIds: number[];
  /** Why the agent picked this offer. */
  rationale: string;
  provider: string | null;
  createdAt: number;
  updatedAt: number;
  syncedAt: number | null;
  remoteId: number | null;
}

export type OfferDraftEdit = Pick<
  OfferDraft,
  "id" | "name" | "promise" | "description" | "priceRange" | "timeline" | "deliverables" | "icpIds" | "caseStudyIds"
>;

export interface OfferRequest {
  provider: string;
  count: number;
  /** Optional steer for the agent; blank lets it pick from all the material. */
  focus: string;
  /** ICPs to base the offers on; empty means all of them. */
  icpIds: number[];
  /** Case studies to base the offers on; empty means all of them. */
  proofIds: number[];
  /** The offer skill the agent follows, or null for the built-in rules. */
  skillId: string | null;
}

export type OfferOutcome =
  | { kind: "created"; offers: OfferDraft[] }
  | { kind: "permissionNeeded"; provider: string; tools: string[] };

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
  /** The platform the idea was planned for: LinkedIn, Facebook, X or Website. */
  channelPlatform: string | null;
  /** The topic the plan was focused on, if one was given. */
  topicTitle: string | null;
  topicDescription: string | null;
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
  /** The cover image's file in the app folder; load it with api.ideaImage. */
  imageFile: string | null;
  /** The idea skill the plan wrote this idea with. */
  skillName: string | null;
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
  /** The channels this idea goes out on. */
  channels: string[];
}

export interface PlanSlot {
  channelId: string;
  channelName: string;
  platform: string;
}

export interface PlanRequest {
  provider: string;
  /** Focus offers in pick order. Slots are shared out across them in turn. */
  offerIds: number[];
  slots: PlanSlot[];
  withImages: boolean;
  /** Draw the images with the image API, not only write their briefs. */
  generateImages: boolean;
  topicTitle: string;
  topicDescription: string;
  /** Planner channel id → idea skill id. Channels left out use the default skill. */
  channelSkills: Record<string, string>;
}

/** A named set of writing instructions sent to the AI agent with a plan. */
export interface Skill {
  id: string;
  name: string;
  content: string;
  isDefault: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface ChannelSkill {
  channelId: string;
  skillId: string;
}

export type ImageProvider = "gemini" | "openai";

export interface ImageApiStatus {
  provider: ImageProvider;
  model: string;
  configured: boolean;
  /** The last characters of the saved key, e.g. "••••x9Qa". */
  keyHint: string | null;
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
  platformPostId?: string | null;
  postUrl?: string | null;
}

export interface MetricValue {
  value: number | null;
  formatted: string;
  available: boolean;
  unavailableReason?: string | null;
}

export interface PostComment {
  id: string;
  author: string;
  authorAvatar?: string | null;
  text: string;
  createdAt?: string | null;
}

export interface PostAnalytics {
  postId: string;
  platformPostId: string | null;
  postUrl: string | null;
  channelId: string;
  platform: string;
  impressions: MetricValue;
  views: MetricValue;
  likes: MetricValue;
  comments: MetricValue;
  latestComments: PostComment[];
  lastUpdated: number;
  error?: string | null;
}

export interface SitePages {
  overview: { overview: Overview | null };
  icps: { icps: SiteIcp[]; offerOptions: Named[] };
  proof: { proof: SiteProof[]; offerOptions: Named[] };
  publish: { channels: PublishChannel[]; posts: SitePost[] };
  queue: { items: QueueItem[]; icps: Named[] };
  pipeline: { stages: PipelineStage[]; cards: PipelineCard[] };
}

/** One row of the Queue: a LinkedIn/Maps lead (a person) or a scanned company (a prospect). */
export interface QueueItem {
  id: number;
  type: "lead" | "prospect";
  company_name: string | null;
  company_domain: string | null;
  first_name: string | null;
  last_name: string | null;
  contact_name: string | null;
  contact_title: string | null;
  headline: string | null;
  about: string | null;
  experience: string[] | null;
  email: string | null;
  phone: string | null;
  website: string | null;
  avatar_url: string | null;
  location: string | null;
  source_label: string;
  linkedin_url: string | null;
  maps_url: string | null;
  category: string | null;
  rating: number | null;
  reviews_count: number | null;
  notes: string | null;
  description: string | null;
  search_label: string | null;
  icp: Named | null;
  enriched_at: string | null;
  company_details: string | null;
  contacts: KeyPerson[] | null;
  researched_at: string | null;
  /** Research found in the app that has not been synced to the website yet. */
  unsynced?: boolean;
  created_at: string;
}

export interface SyncSummary {
  sent: number;
  updated: number;
  fieldsFilled: number;
}

/** A person worth approaching at a company, found by the research agent. */
export interface KeyPerson {
  name: string;
  role: string | null;
  email: string | null;
  phone: string | null;
  linkedin_url: string | null;
}

export interface ResearchPrompt {
  prompt: string;
  isDefault: boolean;
}

/** One queue item picked by hand for research. */
export interface ResearchTarget {
  type: "lead" | "prospect";
  id: number;
}

export interface ResearchOptions {
  provider: string;
  /** The most leads to research in this run; null means all. */
  limit: number | null;
  /** Research leads that were researched before too. */
  includeResearched: boolean;
  /** Research only these items (ignores the limit and history). */
  only: ResearchTarget[];
}

export interface ResearchPlan {
  total: number;
  needing: number;
  /** Leads with gaps that are skipped because they were researched before. */
  alreadyResearched: number;
  fields: { field: string; count: number }[];
}

export type ResearchOutcome =
  | { kind: "done"; checked: number; updated: number; fieldsFilled: number; failed: number; cancelled: boolean }
  | { kind: "permissionNeeded"; provider: string; tools: string[] };

export interface PipelineStage {
  id: number;
  name: string;
  color: string;
  is_default: boolean;
}

export interface PipelineCard {
  key: string;
  id: number;
  type: "lead" | "prospect";
  stage_id: number;
  name: string | null;
  company_name: string | null;
  contact_title?: string | null;
  avatar_url?: string | null;
  company_domain: string | null;
  location: string | null;
  email?: string | null;
  phone?: string | null;
  website?: string | null;
  icp: Named | null;
  source_label: string;
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
  publishIdeas: (ideas: BulkIdeaSchedule[]) => invoke<Idea[]>("publish_ideas", { ideas }),
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
  listSkills: () => invoke<Skill[]>("list_skills"),
  createSkill: (name: string, content: string) => invoke<Skill>("create_skill", { name, content }),
  updateSkill: (id: string, name: string, content: string) => invoke<Skill>("update_skill", { id, name, content }),
  deleteSkill: (id: string) => invoke<void>("delete_skill", { id }),
  setDefaultSkill: (id: string) => invoke<void>("set_default_skill", { id }),
  skillTemplate: () => invoke<string>("skill_template"),
  getResearchPrompt: () => invoke<ResearchPrompt>("get_research_prompt"),
  saveResearchPrompt: (prompt: string) => invoke<ResearchPrompt>("save_research_prompt", { prompt }),
  resetResearchPrompt: () => invoke<ResearchPrompt>("reset_research_prompt"),
  pendingResearchCount: () => invoke<number>("pending_research_count"),
  /** Sends the research found so far to the website. */
  syncLeadResearch: () => invoke<SyncSummary>("sync_lead_research"),
  researchPlan: (includeResearched: boolean, only: ResearchTarget[]) =>
    invoke<ResearchPlan>("research_plan", { includeResearched, only }),
  researchLeads: (options: ResearchOptions) => invoke<ResearchOutcome>("research_leads", { request: options }),
  listOfferSkills: () => invoke<Skill[]>("list_offer_skills"),
  createOfferSkill: (name: string, content: string) => invoke<Skill>("create_offer_skill", { name, content }),
  updateOfferSkill: (id: string, name: string, content: string) => invoke<Skill>("update_offer_skill", { id, name, content }),
  deleteOfferSkill: (id: string) => invoke<void>("delete_offer_skill", { id }),
  offerSkillTemplate: () => invoke<string>("offer_skill_template"),
  listChannelSkills:() => invoke<ChannelSkill[]>("list_channel_skills"),
  setChannelSkill: (channelId: string, skillId: string | null) => invoke<void>("set_channel_skill", { channelId, skillId }),
  getImageApi: () => invoke<ImageApiStatus>("get_image_api"),
  saveImageApi: (provider: ImageProvider, model: string, apiKey: string | null) =>
    invoke<ImageApiStatus>("save_image_api", { provider, model, apiKey }),
  removeImageApi: () => invoke<ImageApiStatus>("remove_image_api"),
  testImageApi: () => invoke<string>("test_image_api"),
  ideaImage: (id: string) => invoke<string | null>("idea_image", { id }),
  setIdeaImage: (id: string, dataUrl: string) => invoke<Idea>("set_idea_image", { id, dataUrl }),
  removeIdeaImage: (id: string) => invoke<Idea>("remove_idea_image", { id }),
  generateIdeaImage: (id: string, prompt: string | null) => invoke<Idea>("generate_idea_image", { id, prompt }),
  listIdeas: (tab: IdeaTab, page = 1) => invoke<IdeaPage>("list_ideas", { tab, page }),
  addIdea: (title: string) => invoke<Idea>("add_idea", { title }),
  deleteIdea: (id: string) => invoke<void>("delete_idea", { id }),
  generatePlan: (request: PlanRequest) => invoke<PlanOutcome>("generate_plan", { request }),
  /** Progress arrives as `plan-progress` events; `cancelPlan` stops it. */
  generateOffers: (request: OfferRequest) => invoke<OfferOutcome>("generate_offers", { request }),
  listOfferDrafts: () => invoke<OfferDraft[]>("list_offer_drafts"),
  updateOfferDraft: (draft: OfferDraftEdit) => invoke<OfferDraft>("update_offer_draft", { draft }),
  deleteOfferDraft: (id: string) => invoke<void>("delete_offer_draft", { id }),
  /** Puts every draft not synced yet on the main site; resolves to how many went. */
  syncOfferDrafts: () => invoke<number>("sync_offer_drafts"),
  fetchPostAnalytics: (postId: string, channelId: string, platformPostId?: string | null, postUrl?: string | null) =>
    invoke<PostAnalytics>("fetch_post_analytics", {
      postId,
      channelId,
      platformPostId: platformPostId ?? null,
      postUrl: postUrl ?? null,
    }),
  getPostAnalytics: (postId: string) => invoke<PostAnalytics | null>("get_post_analytics", { postId }),
  setPostPlatformId: (postId: string, platformPostId: string, postUrl?: string | null) =>
    invoke<void>("set_post_platform_id", { postId, platformPostId, postUrl: postUrl ?? null }),
  getChannelTokens: () => invoke<Record<string, string>>("get_channel_tokens"),
  saveChannelToken: (channelId: string, token: string) => invoke<void>("save_channel_token", { channelId, token }),
};

/** Tauri commands reject with a plain string; anything else is unexpected. */
export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}
