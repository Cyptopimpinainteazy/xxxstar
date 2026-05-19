
// KING team and superuser bootstrap
export const bootstrapKing = async () => {
  // Register King user if not exists
  try {
    await register("King", "King", "king@x3star.net", "x3star", "KING");
  } catch (e) {
    // Ignore if already exists
  }
  // Create KING team if not exists
  try {
    const session = await login("King", "x3star");
    await createGroup(session.userId, "KING", "Secret admin team for King", "admin");
  } catch (e) {
    // Ignore if already exists
  }
};
/**
 * socialService.ts — Tauri invoke wrappers for the social network backend.
 * Every function maps 1:1 to a Rust #[tauri::command].
 */
// Lazy guarded tauri invoke helper
async function invoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
} 

/* ─── Types ──────────────────────────────────────── */

export interface AuthSession {
  userId: string;
  username: string;
  token: string;
  role: string;
}

export interface User {
  id: string;
  username: string;
  displayName: string;
  email: string;
  avatarUrl: string;
  headline: string;
  aboutMe: string;
  whoIdLikeToMeet: string;
  interests: string;
  musicInterests: string;
  movieInterests: string;
  heroSongPath: string;
  heroSongTitle: string;
  profileCss: string;
  profileBgUrl: string;
  mood: string;
  gender: string;
  age: number;
  location: string;
  orientation: string;
  status: string;
  bodyType: string;
  ethnicity: string;
  zodiacSign: string;
  smokeDrink: string;
  children: string;
  education: string;
  occupation: string;
  income: string;
  onlineStatus: string;
  lastLogin: string;
  profileViews: number;
  role: string;
  createdAt: string;
  updatedAt: string;
}

export interface UpdateProfileInput {
  displayName?: string;
  avatarUrl?: string;
  headline?: string;
  aboutMe?: string;
  whoIdLikeToMeet?: string;
  interests?: string;
  musicInterests?: string;
  movieInterests?: string;
  heroSongPath?: string;
  heroSongTitle?: string;
  profileCss?: string;
  profileBgUrl?: string;
  mood?: string;
  gender?: string;
  age?: number;
  location?: string;
  orientation?: string;
  status?: string;
  bodyType?: string;
  ethnicity?: string;
  zodiacSign?: string;
  smokeDrink?: string;
  children?: string;
  education?: string;
  occupation?: string;
  income?: string;
}

export interface Friend {
  id: string;
  userId: string;
  username: string;
  displayName: string;
  avatarUrl: string;
  headline: string;
  onlineStatus: string;
  isTopFriend: boolean;
  topFriendRank: number;
}

export interface FriendRequest {
  id: string;
  fromUserId: string;
  fromUsername: string;
  fromDisplayName: string;
  fromAvatar: string;
  toUserId: string;
  status: string;
  createdAt: string;
}

export interface Message {
  id: string;
  fromUserId: string;
  fromUsername: string;
  fromDisplayName: string;
  fromAvatar: string;
  toUserId: string;
  subject: string;
  body: string;
  isRead: boolean;
  createdAt: string;
}

export interface Bulletin {
  id: string;
  userId: string;
  username: string;
  displayName: string;
  avatarUrl: string;
  title: string;
  body: string;
  createdAt: string;
}

export interface ProfileComment {
  id: string;
  profileUserId: string;
  authorUserId: string;
  authorUsername: string;
  authorDisplayName: string;
  authorAvatar: string;
  body: string;
  createdAt: string;
}

export interface BlogPost {
  id: string;
  userId: string;
  username: string;
  displayName: string;
  avatarUrl: string;
  title: string;
  body: string;
  mood: string;
  comments: BlogComment[];
  createdAt: string;
  updatedAt: string;
}

export interface BlogComment {
  id: string;
  authorUserId: string;
  authorUsername: string;
  authorDisplayName: string;
  authorAvatar: string;
  body: string;
  createdAt: string;
}

export interface Photo {
  id: string;
  userId: string;
  albumName: string;
  filePath: string;
  caption: string;
  isDefault: boolean;
  createdAt: string;
}

export interface MusicTrack {
  id: string;
  userId: string;
  title: string;
  artist: string;
  filePath: string;
  durationSecs: number;
  playCount: number;
  isProfileSong: boolean;
  createdAt: string;
}

export interface StatusUpdate {
  id: string;
  userId: string;
  username: string;
  displayName: string;
  avatarUrl: string;
  body: string;
  createdAt: string;
}

export interface Kudo {
  id: string;
  fromUserId: string;
  fromUsername: string;
  fromAvatar: string;
  kind: string;
  createdAt: string;
}

export interface UserSearchResult {
  id: string;
  username: string;
  displayName: string;
  avatarUrl: string;
  headline: string;
  location: string;
  onlineStatus: string;
}

export interface Group {
  id: string;
  name: string;
  description: string;
  ownerUserId: string;
  category: string;
  avatarUrl: string;
  memberCount: number;
  createdAt: string;
}

export interface SocialStats {
  friendCount: number;
  pendingRequests: number;
  unreadMessages: number;
  photoCount: number;
  musicCount: number;
  profileViews: number;
  bulletinCount: number;
  blogCount: number;
  kudoCount: number;
}

/* ─── Auth ───────────────────────────────────────── */
export const register = (username: string, displayName: string, email: string, password: string, teamCode?: string) =>
  invoke<AuthSession>("social_register", { input: { username, displayName, email, password, teamCode: teamCode || null } });

export const login = (username: string, password: string) =>
  invoke<AuthSession>("social_login", { input: { username, password } });

export const logout = (userId: string) =>
  invoke<void>("social_logout", { userId });

export const validateTeamCode = (code: string) =>
  invoke<string>("social_validate_team_code", { code });

export interface TeamCode {
  id: string;
  code: string;
  label: string;
  role: string;
  maxUses: number;
  useCount: number;
  active: boolean;
  createdAt: string;
}

export const getTeamCodes = (userId: string) =>
  invoke<TeamCode[]>("social_get_team_codes", { userId });

export const createTeamCode = (userId: string, code: string, label: string, role: string, maxUses?: number) =>
  invoke<TeamCode>("social_create_team_code", { userId, code, label, role, maxUses: maxUses ?? 0 });


/* ─── Profile ────────────────────────────────────── */
export const getProfile = (userId: string, viewerId?: string) =>
  invoke<User>("social_get_profile", { userId, viewerId });

export const getProfileByUsername = (username: string) =>
  invoke<User>("social_get_profile_by_username", { username });

export const updateProfile = (userId: string, input: UpdateProfileInput) =>
  invoke<User>("social_update_profile", { userId, input });

/* ─── Friends ────────────────────────────────────── */
export const sendFriendRequest = (fromUserId: string, toUserId: string) =>
  invoke<string>("social_send_friend_request", { fromUserId, toUserId });

export const respondFriendRequest = (requestId: string, accept: boolean) =>
  invoke<void>("social_respond_friend_request", { requestId, accept });

export const getFriends = (userId: string) =>
  invoke<Friend[]>("social_get_friends", { userId });

export const getPendingRequests = (userId: string) =>
  invoke<FriendRequest[]>("social_get_pending_requests", { userId });

export const setTopFriends = (userId: string, friendIds: string[]) =>
  invoke<void>("social_set_top_friends", { userId, friendIds });

export const removeFriend = (userId: string, friendId: string) =>
  invoke<void>("social_remove_friend", { userId, friendId });

/* ─── Messages ───────────────────────────────────── */
export const sendMessage = (fromUserId: string, toUserId: string, subject: string, body: string) =>
  invoke<string>("social_send_message", { fromUserId, input: { toUserId, subject, body } });

export const getInbox = (userId: string) =>
  invoke<Message[]>("social_get_inbox", { userId });

export const getSentMessages = (userId: string) =>
  invoke<Message[]>("social_get_sent_messages", { userId });

export const markMessageRead = (messageId: string) =>
  invoke<void>("social_mark_message_read", { messageId });

export const deleteMessage = (messageId: string) =>
  invoke<void>("social_delete_message", { messageId });

/* ─── Bulletins ──────────────────────────────────── */
export const postBulletin = (userId: string, title: string, body: string) =>
  invoke<string>("social_post_bulletin", { userId, input: { title, body } });

export const getBulletins = (userId: string) =>
  invoke<Bulletin[]>("social_get_bulletins", { userId });

/* ─── Comments ───────────────────────────────────── */
export const postComment = (authorUserId: string, profileUserId: string, body: string) =>
  invoke<string>("social_post_comment", { authorUserId, input: { profileUserId, body } });

export const getComments = (profileUserId: string) =>
  invoke<ProfileComment[]>("social_get_comments", { profileUserId });

export const deleteComment = (commentId: string, userId: string) =>
  invoke<void>("social_delete_comment", { commentId, userId });

/* ─── Blog ───────────────────────────────────────── */
export const createBlogPost = (userId: string, title: string, body: string, mood?: string) =>
  invoke<string>("social_create_blog_post", { userId, input: { title, body, mood } });

export const getBlogPosts = (userId: string) =>
  invoke<BlogPost[]>("social_get_blog_posts", { userId });

export const postBlogComment = (authorUserId: string, blogPostId: string, body: string) =>
  invoke<string>("social_post_blog_comment", { authorUserId, input: { blogPostId, body } });

/* ─── Photos ─────────────────────────────────────── */
export const addPhoto = (userId: string, filePath: string, caption: string, album?: string) =>
  invoke<string>("social_add_photo", { userId, filePath, caption, album });

export const getPhotos = (userId: string) =>
  invoke<Photo[]>("social_get_photos", { userId });

export const deletePhoto = (photoId: string, userId: string) =>
  invoke<void>("social_delete_photo", { photoId, userId });

/* ─── Music ──────────────────────────────────────── */
export const addMusic = (userId: string, title: string, artist: string, filePath: string) =>
  invoke<string>("social_add_music", { userId, title, artist, filePath });

export const getMusic = (userId: string) =>
  invoke<MusicTrack[]>("social_get_music", { userId });

export const setProfileSong = (userId: string, trackId: string) =>
  invoke<void>("social_set_profile_song", { userId, trackId });

/* ─── Feed / Status ──────────────────────────────── */
export const postStatus = (userId: string, body: string) =>
  invoke<string>("social_post_status", { userId, input: { body } });

export const getFeed = (userId: string) =>
  invoke<StatusUpdate[]>("social_get_feed", { userId });

/* ─── Search ─────────────────────────────────────── */
export const searchUsers = (query: string) =>
  invoke<UserSearchResult[]>("social_search_users", { query });

export const browseUsers = (offset?: number) =>
  invoke<UserSearchResult[]>("social_browse_users", { offset });

/* ─── Kudos ──────────────────────────────────────── */
export const sendKudo = (fromUserId: string, toUserId: string, kind: string) =>
  invoke<string>("social_send_kudo", { fromUserId, toUserId, kind });

export const getKudos = (userId: string) =>
  invoke<Kudo[]>("social_get_kudos", { userId });

/* ─── Groups ─────────────────────────────────────── */
export const createGroup = (userId: string, name: string, description: string, category?: string) =>
  invoke<string>("social_create_group", { userId, input: { name, description, category } });

export const getGroups = () =>
  invoke<Group[]>("social_get_groups");

export const joinGroup = (userId: string, groupId: string) =>
  invoke<void>("social_join_group", { userId, groupId });

/* ─── Stats ──────────────────────────────────────── */
export const getStats = (userId: string) =>
  invoke<SocialStats>("social_get_stats", { userId });
