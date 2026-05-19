/**
 * socialStore.ts — Global social network state via Zustand.
 */
import { create } from "zustand";
import * as api from "@/services/socialService";
import type {
  AuthSession, User, Friend, FriendRequest, Message,
  Bulletin, ProfileComment, StatusUpdate, SocialStats,
  MusicTrack, Photo, UserSearchResult, Kudo, BlogPost, Group,
  UpdateProfileInput,
} from "@/services/socialService";

interface SocialState {
  // auth
  session: AuthSession | null;
  currentUser: User | null;
  isLoggedIn: boolean;

  // data
  friends: Friend[];
  pendingRequests: FriendRequest[];
  inbox: Message[];
  sentMessages: Message[];
  bulletins: Bulletin[];
  comments: ProfileComment[];
  feed: StatusUpdate[];
  stats: SocialStats | null;
  music: MusicTrack[];
  photos: Photo[];
  searchResults: UserSearchResult[];
  kudos: Kudo[];
  blogPosts: BlogPost[];
  groups: Group[];
  browseResults: UserSearchResult[];

  // viewed profile
  viewedProfile: User | null;
  viewedFriends: Friend[];
  viewedComments: ProfileComment[];
  viewedPhotos: Photo[];
  viewedMusic: MusicTrack[];
  viewedBlogPosts: BlogPost[];
  viewedKudos: Kudo[];

  // loading
  loading: boolean;
  error: string | null;

  // actions
  register: (username: string, displayName: string, email: string, password: string, teamCode?: string) => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  restoreSession: () => void;
  
  loadProfile: () => Promise<void>;
  updateProfile: (input: UpdateProfileInput) => Promise<void>;
  
  loadFriends: () => Promise<void>;
  loadPendingRequests: () => Promise<void>;
  sendFriendRequest: (toUserId: string) => Promise<void>;
  respondFriendRequest: (requestId: string, accept: boolean) => Promise<void>;
  setTopFriends: (friendIds: string[]) => Promise<void>;
  removeFriend: (friendId: string) => Promise<void>;

  loadInbox: () => Promise<void>;
  loadSentMessages: () => Promise<void>;
  sendMessage: (toUserId: string, subject: string, body: string) => Promise<void>;
  markRead: (messageId: string) => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;

  loadBulletins: () => Promise<void>;
  postBulletin: (title: string, body: string) => Promise<void>;

  loadComments: (profileUserId: string) => Promise<void>;
  postComment: (profileUserId: string, body: string) => Promise<void>;
  deleteComment: (commentId: string) => Promise<void>;

  loadFeed: () => Promise<void>;
  postStatus: (body: string) => Promise<void>;

  loadStats: () => Promise<void>;

  loadMusic: () => Promise<void>;
  addMusic: (title: string, artist: string, filePath: string) => Promise<void>;
  setProfileSong: (trackId: string) => Promise<void>;

  loadPhotos: () => Promise<void>;
  addPhoto: (filePath: string, caption: string, album?: string) => Promise<void>;
  deletePhoto: (photoId: string) => Promise<void>;

  search: (query: string) => Promise<void>;
  browse: (offset?: number) => Promise<void>;

  sendKudo: (toUserId: string, kind: string) => Promise<void>;
  loadKudos: (userId: string) => Promise<void>;

  createBlogPost: (title: string, body: string, mood?: string) => Promise<void>;
  loadBlogPosts: (userId: string) => Promise<void>;
  postBlogComment: (blogPostId: string, body: string) => Promise<void>;

  createGroup: (name: string, description: string, category?: string) => Promise<void>;
  loadGroups: () => Promise<void>;
  joinGroup: (groupId: string) => Promise<void>;

  // View other profile
  viewProfile: (userId: string) => Promise<void>;
  viewProfileByUsername: (username: string) => Promise<void>;

  // wallet integration
  loginWithWallet: (address: string) => Promise<void>;
}

const STORAGE_KEY = "x3-social-session";
const hasTauriRuntime = () =>
  typeof window !== "undefined" && (((window as any).__TAURI_INTERNALS__) || ((window as any).__TAURI__));

export const useSocialStore = create<SocialState>((set, get) => ({
  session: null,
  currentUser: null,
  isLoggedIn: false,
  friends: [],
  pendingRequests: [],
  inbox: [],
  sentMessages: [],
  bulletins: [],
  comments: [],
  feed: [],
  stats: null,
  music: [],
  photos: [],
  searchResults: [],
  kudos: [],
  blogPosts: [],
  groups: [],
  browseResults: [],
  viewedProfile: null,
  viewedFriends: [],
  viewedComments: [],
  viewedPhotos: [],
  viewedMusic: [],
  viewedBlogPosts: [],
  viewedKudos: [],
  loading: false,
  error: null,

  /* ─── Auth ─────────────────────────────────────── */
  register: async (username, displayName, email, password, teamCode) => {
    set({ loading: true, error: null });
    try {
      const session = await api.register(username, displayName, email, password, teamCode);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
      set({ session, isLoggedIn: true, loading: false });
      await get().loadProfile();
      await get().loadStats();
    } catch (err: any) {
      set({ error: String(err), loading: false });
    }
  },

  login: async (username, password) => {
    set({ loading: true, error: null });
    try {
      const session = await api.login(username, password);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
      set({ session, isLoggedIn: true, loading: false });
      await get().loadProfile();
      await get().loadStats();
    } catch (err: any) {
      set({ error: String(err), loading: false });
    }
  },

  logout: async () => {
    const s = get().session;
    if (s) await api.logout(s.userId).catch(() => {});
    localStorage.removeItem(STORAGE_KEY);
    set({
      session: null, currentUser: null, isLoggedIn: false,
      friends: [], pendingRequests: [], inbox: [], sentMessages: [],
      bulletins: [], comments: [], feed: [], stats: null,
      music: [], photos: [], searchResults: [], kudos: [],
      blogPosts: [], groups: [], browseResults: [],
      viewedProfile: null, viewedFriends: [], viewedComments: [],
      viewedPhotos: [], viewedMusic: [], viewedBlogPosts: [], viewedKudos: [],
    });
  },

  restoreSession: () => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const session: AuthSession = JSON.parse(raw);
        set({ session, isLoggedIn: true });
        if (hasTauriRuntime()) {
          get().loadProfile();
          get().loadStats();
        }
      }
    } catch { /* ignore */ }
  },

  /* ─── Profile ──────────────────────────────────── */
  loadProfile: async () => {
    const s = get().session;
    if (!s) return;
    try {
      const user = await api.getProfile(s.userId);
      set({ currentUser: user });
    } catch (err: any) {
      set({ error: String(err) });
    }
  },

  updateProfile: async (input) => {
    const s = get().session;
    if (!s) return;
    set({ loading: true });
    try {
      const user = await api.updateProfile(s.userId, input);
      set({ currentUser: user, loading: false });
    } catch (err: any) {
      set({ error: String(err), loading: false });
    }
  },

  /* ─── Friends ──────────────────────────────────── */
  loadFriends: async () => {
    const s = get().session;
    if (!s) return;
    const friends = await api.getFriends(s.userId);
    set({ friends });
  },

  loadPendingRequests: async () => {
    const s = get().session;
    if (!s) return;
    const pendingRequests = await api.getPendingRequests(s.userId);
    set({ pendingRequests });
  },

  sendFriendRequest: async (toUserId) => {
    const s = get().session;
    if (!s) return;
    await api.sendFriendRequest(s.userId, toUserId);
  },

  respondFriendRequest: async (requestId, accept) => {
    await api.respondFriendRequest(requestId, accept);
    await get().loadPendingRequests();
    await get().loadFriends();
    await get().loadStats();
  },

  setTopFriends: async (friendIds) => {
    const s = get().session;
    if (!s) return;
    await api.setTopFriends(s.userId, friendIds);
    await get().loadFriends();
  },

  removeFriend: async (friendId) => {
    const s = get().session;
    if (!s) return;
    await api.removeFriend(s.userId, friendId);
    await get().loadFriends();
    await get().loadStats();
  },

  /* ─── Messages ─────────────────────────────────── */
  loadInbox: async () => {
    const s = get().session;
    if (!s) return;
    const inbox = await api.getInbox(s.userId);
    set({ inbox });
  },

  loadSentMessages: async () => {
    const s = get().session;
    if (!s) return;
    const sentMessages = await api.getSentMessages(s.userId);
    set({ sentMessages });
  },

  sendMessage: async (toUserId, subject, body) => {
    const s = get().session;
    if (!s) return;
    await api.sendMessage(s.userId, toUserId, subject, body);
  },

  markRead: async (messageId) => {
    await api.markMessageRead(messageId);
    await get().loadInbox();
    await get().loadStats();
  },

  deleteMessage: async (messageId) => {
    await api.deleteMessage(messageId);
    await get().loadInbox();
  },

  /* ─── Bulletins ────────────────────────────────── */
  loadBulletins: async () => {
    const s = get().session;
    if (!s) return;
    const bulletins = await api.getBulletins(s.userId);
    set({ bulletins });
  },

  postBulletin: async (title, body) => {
    const s = get().session;
    if (!s) return;
    await api.postBulletin(s.userId, title, body);
    await get().loadBulletins();
  },

  /* ─── Comments ─────────────────────────────────── */
  loadComments: async (profileUserId) => {
    const comments = await api.getComments(profileUserId);
    set({ viewedComments: comments });
  },

  postComment: async (profileUserId, body) => {
    const s = get().session;
    if (!s) return;
    await api.postComment(s.userId, profileUserId, body);
    await get().loadComments(profileUserId);
  },

  deleteComment: async (commentId) => {
    const s = get().session;
    if (!s) return;
    await api.deleteComment(commentId, s.userId);
  },

  /* ─── Feed ─────────────────────────────────────── */
  loadFeed: async () => {
    const s = get().session;
    if (!s) return;
    const feed = await api.getFeed(s.userId);
    set({ feed });
  },

  postStatus: async (body) => {
    const s = get().session;
    if (!s) return;
    await api.postStatus(s.userId, body);
    await get().loadFeed();
  },

  /* ─── Stats ────────────────────────────────────── */
  loadStats: async () => {
    const s = get().session;
    if (!s) return;
    const stats = await api.getStats(s.userId);
    set({ stats });
  },

  /* ─── Music ────────────────────────────────────── */
  loadMusic: async () => {
    const s = get().session;
    if (!s) return;
    const music = await api.getMusic(s.userId);
    set({ music });
  },

  addMusic: async (title, artist, filePath) => {
    const s = get().session;
    if (!s) return;
    await api.addMusic(s.userId, title, artist, filePath);
    await get().loadMusic();
  },

  setProfileSong: async (trackId) => {
    const s = get().session;
    if (!s) return;
    await api.setProfileSong(s.userId, trackId);
    await get().loadMusic();
  },

  /* ─── Photos ───────────────────────────────────── */
  loadPhotos: async () => {
    const s = get().session;
    if (!s) return;
    const photos = await api.getPhotos(s.userId);
    set({ photos });
  },

  addPhoto: async (filePath, caption, album) => {
    const s = get().session;
    if (!s) return;
    await api.addPhoto(s.userId, filePath, caption, album);
    await get().loadPhotos();
  },

  deletePhoto: async (photoId) => {
    const s = get().session;
    if (!s) return;
    await api.deletePhoto(photoId, s.userId);
    await get().loadPhotos();
  },

  /* ─── Search ───────────────────────────────────── */
  search: async (query) => {
    const searchResults = await api.searchUsers(query);
    set({ searchResults });
  },

  browse: async (offset) => {
    const browseResults = await api.browseUsers(offset);
    set({ browseResults });
  },

  /* ─── Kudos ────────────────────────────────────── */
  sendKudo: async (toUserId, kind) => {
    const s = get().session;
    if (!s) return;
    await api.sendKudo(s.userId, toUserId, kind);
  },

  loadKudos: async (userId) => {
    const kudos = await api.getKudos(userId);
    set({ viewedKudos: kudos });
  },

  /* ─── Blog ─────────────────────────────────────── */
  createBlogPost: async (title, body, mood) => {
    const s = get().session;
    if (!s) return;
    await api.createBlogPost(s.userId, title, body, mood);
    await get().loadBlogPosts(s.userId);
  },

  loadBlogPosts: async (userId) => {
    const blogPosts = await api.getBlogPosts(userId);
    set({ viewedBlogPosts: blogPosts });
  },

  postBlogComment: async (blogPostId, body) => {
    const s = get().session;
    if (!s) return;
    await api.postBlogComment(s.userId, blogPostId, body);
  },

  /* ─── Groups ───────────────────────────────────── */
  createGroup: async (name, description, category) => {
    const s = get().session;
    if (!s) return;
    await api.createGroup(s.userId, name, description, category);
    await get().loadGroups();
  },

  loadGroups: async () => {
    const groups = await api.getGroups();
    set({ groups });
  },

  joinGroup: async (groupId) => {
    const s = get().session;
    if (!s) return;
    await api.joinGroup(s.userId, groupId);
    await get().loadGroups();
  },

  /* ─── View Other Profile ───────────────────────── */
  viewProfile: async (userId) => {
    const s = get().session;
    try {
      const [profile, friends, comments, photos, music, blogPosts, kudos] = await Promise.all([
        api.getProfile(userId, s?.userId),
        api.getFriends(userId),
        api.getComments(userId),
        api.getPhotos(userId),
        api.getMusic(userId),
        api.getBlogPosts(userId),
        api.getKudos(userId),
      ]);
      set({
        viewedProfile: profile,
        viewedFriends: friends,
        viewedComments: comments,
        viewedPhotos: photos,
        viewedMusic: music,
        viewedBlogPosts: blogPosts,
        viewedKudos: kudos,
      });
    } catch (err: any) {
      set({ error: String(err) });
    }
  },

  viewProfileByUsername: async (username) => {
    try {
      const profile = await api.getProfileByUsername(username);
      await get().viewProfile(profile.id);
    } catch (err: any) {
      set({ error: String(err) });
    }
  },

  loginWithWallet: async (address) => {
    set({ loading: true, error: null });
    try {
      // Create a synthetic session based on wallet address
      const session: AuthSession = {
        userId: address,
        username: address,
        token: `wallet:${address}`,
        role: "user",
      };
      
      // Create a synthetic user profile
      const user: User = {
        id: address,
        username: address,
        displayName: `${address.slice(0, 6)}...${address.slice(-4)}`,
        email: `${address.slice(0, 8)}@x3.internal`,
        avatarUrl: "",
        headline: "X3 Universal Wallet User",
        aboutMe: "",
        whoIdLikeToMeet: "",
        interests: "",
        musicInterests: "",
        movieInterests: "",
        heroSongPath: "",
        heroSongTitle: "",
        profileCss: "",
        profileBgUrl: "",
        mood: "connected",
        gender: "unknown",
        age: 0,
        location: "X3 Chain",
        orientation: "",
        status: "Active",
        bodyType: "",
        ethnicity: "",
        zodiacSign: "",
        smokeDrink: "",
        children: "",
        education: "",
        occupation: "Web3 Citizen",
        income: "",
        onlineStatus: "online",
        lastLogin: new Date().toISOString(),
        profileViews: 0,
        role: "user",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
      set({ session, currentUser: user, isLoggedIn: true, loading: false });
      
      // Attempt to load stats (will likely fail for new wallet users, but that's fine)
      await get().loadStats().catch(() => {});
    } catch (err: any) {
      set({ error: String(err), loading: false });
    }
  },
}));
