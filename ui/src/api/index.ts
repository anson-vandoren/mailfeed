import { user, addToast, appState } from "../stores";
import { get } from "svelte/store";
import axios from "axios";
import type { AxiosResponse } from "axios";
import type { StoredUser } from "../stores";
import { API_BASE_URL } from "../lib/config";
import { goto } from "$app/navigation";

// Configure axios to send cookies with all requests
axios.defaults.withCredentials = true;

// Global axios interceptor for handling auth errors and network issues
axios.interceptors.request.use(
  (config) => {
    // Set loading state
    appState.update(state => ({ ...state, isLoading: true }));
    console.log('Making API request:', config.method?.toUpperCase(), config.url);
    return config;
  },
  (error) => {
    appState.update(state => ({ ...state, isLoading: false }));
    console.error('Request setup error:', error);
    return Promise.reject(error);
  }
);

axios.interceptors.response.use(
  (response) => {
    // Clear loading state on success
    appState.update(state => ({ ...state, isLoading: false }));
    console.log('API response:', response.status, response.config.url);
    return response;
  },
  (error) => {
    // Clear loading state on error
    appState.update(state => ({ ...state, isLoading: false }));
    
    console.error('API Error:', {
      url: error.config?.url,
      method: error.config?.method,
      status: error.response?.status,
      statusText: error.response?.statusText,
      data: error.response?.data,
      message: error.message
    });
    
    // Handle different error types
    if (error.response?.status === 401 || error.response?.status === 403) {
      console.log('Authentication failed, clearing user session');
      addToast({
        message: 'Your session has expired. Please log in again.',
        type: 'warning'
      });
      // Clear user session
      user.set({});
      // Redirect to login
      goto('/');
    } else if (error.code === 'NETWORK_ERROR' || !error.response) {
      addToast({
        message: 'Network error. Please check your connection.',
        type: 'error'
      });
    } else if (error.response?.status >= 500) {
      addToast({
        message: 'Server error. Please try again later.',
        type: 'error'
      });
    } else if (error.response?.status === 404) {
      addToast({
        message: 'Endpoint not found. Check if the backend is running.',
        type: 'error'
      });
    }
    
    return Promise.reject(error);
  }
);

export function login(email: string, password: string): Promise<AxiosResponse> {
  return axios.post(`${API_BASE_URL}/api/auth/login`, { email, password });
}

export async function logout(): Promise<void> {
  // Clear user state immediately
  user.set({});
  
  // Call backend logout endpoint (session cookie will be automatically sent)
  try {
    await axios.post(`${API_BASE_URL}/api/auth/logout`);
  } catch (e) {
    console.log('Backend logout failed (expected if session was already invalid)');
  }
  
  // Redirect to login page
  goto('/');
}

type UserDetails = {
  id: number;
  loginEmail: string;
  sendEmail: string;
  createdAt: number;
  dailySendTime: string;
  telegram_chat_id?: string;
  telegram_username?: string;
};
export async function getUserDetails(localUser: StoredUser): Promise<UserDetails> {
  const { userId } = localUser;
  console.log(localUser)
  if (!userId) throw new Error("No userId");

  const res = await axios.get(`${API_BASE_URL}/api/users/${userId}`);

    return {
      id: res.data.id,
      loginEmail: res.data.login_email,
      sendEmail: res.data.send_email,
      createdAt: res.data.created_at,
      dailySendTime: res.data.daily_send_time,
      telegram_chat_id: res.data.telegram_chat_id,
      telegram_username: res.data.telegram_username,
    }
}

// Subscription management
export type Frequency = 'realtime' | 'hourly' | 'daily';

export interface Subscription {
  id: number;
  user_id: number;
  friendly_name: string;
  frequency: Frequency;
  last_sent_time: number;
  max_items: number;
  is_active: boolean;
  feed_id: number;
}

export interface Feed {
  id: number;
  url: string;
  feed_type: number;
  title: string;
  last_checked: number;
  last_updated: number;
  error_time: number;
  error_message?: string;
}

export interface SubscriptionWithFeed {
  subscription: Subscription;
  feed: Feed;
}

export interface UpdateSubscriptionRequest {
  frequency?: Frequency;
  friendly_name?: string;
  max_items?: number;
  is_active?: boolean;
}

export interface CreateSubscriptionRequest {
  url: string;
  frequency: Frequency;
  friendly_name?: string;
  max_items?: number;
}

export interface ValidateFeedRequest {
  url: string;
}

export interface ValidateFeedResponse {
  valid: boolean;
  title?: string;
  description?: string;
  error?: string;
}

export async function getSubscriptions(localUser: StoredUser): Promise<SubscriptionWithFeed[]> {
  const res = await axios.get(`${API_BASE_URL}/api/subscriptions`);

  return res.data;
}

export async function createSubscription(
  localUser: StoredUser, 
  subscription: CreateSubscriptionRequest
): Promise<SubscriptionWithFeed> {
  const res = await axios.post(
    `${API_BASE_URL}/api/subscriptions`,
    subscription,
  );

  return res.data;
}

export async function updateSubscription(
  localUser: StoredUser,
  subscriptionId: number,
  updates: UpdateSubscriptionRequest
): Promise<SubscriptionWithFeed> {
  const res = await axios.patch(
    `${API_BASE_URL}/api/subscriptions/${subscriptionId}`,
    updates,
  );

  return res.data;
}

export async function deleteSubscription(
  localUser: StoredUser,
  subscriptionId: number
): Promise<void> {
  await axios.delete(
    `${API_BASE_URL}/api/subscriptions/${subscriptionId}`,
  );
}

export async function validateFeed(
  localUser: StoredUser,
  url: string
): Promise<ValidateFeedResponse> {
  // Session cookie will be automatically sent

  const res = await axios.post(
    `${API_BASE_URL}/api/feeds/validate`,
    { url },
  );

  return res.data;
}

export async function updateTelegramSettings(
  localUser: StoredUser,
  telegramChatId: string,
  telegramUsername?: string
): Promise<UserDetails> {
  const { userId } = localUser;
  if (!userId) throw new Error("No userId");

  const res = await axios.patch(
    `${API_BASE_URL}/api/users/${userId}`,
    {
      telegram_chat_id: telegramChatId,
      telegram_username: telegramUsername
    },
  );

  return {
    id: res.data.id,
    loginEmail: res.data.login_email,
    sendEmail: res.data.send_email,
    createdAt: res.data.created_at,
    dailySendTime: res.data.daily_send_time,
  };
}

export async function testTelegram(localUser: StoredUser): Promise<any> {
  const { userId } = localUser;
  if (!userId) throw new Error("No userId");

  const res = await axios.get(
    `${API_BASE_URL}/api/users/${userId}/test-telegram`,
  );

  return res.data;
}

// Configuration management
export interface ConfigItem {
  key: string;
  value: string;
  description?: string;
  config_type: 'string' | 'number' | 'boolean' | 'select';
  category: 'telegram' | 'feed_monitoring' | 'message_delivery' | 'authentication' | 'system';
  updated_at: number;
}

export interface ConfigSchema {
  key: string;
  display_name: string;
  description: string;
  config_type: 'string' | 'number' | 'boolean' | 'select';
  category: 'telegram' | 'feed_monitoring' | 'message_delivery' | 'authentication' | 'system';
  default_value: string;
  validation?: {
    min?: number;
    max?: number;
    pattern?: string;
    required: boolean;
  };
  options?: Array<{
    value: string;
    label: string;
  }>;
}

export interface ConfigResponse {
  config: Record<string, ConfigItem>;
  schema: ConfigSchema[];
}

export async function getConfiguration(localUser: StoredUser): Promise<ConfigResponse> {
  const { userId } = localUser;
  if (!userId) throw new Error("No userId");

  const res = await axios.get(
    `${API_BASE_URL}/api/users/${userId}/config`,
  );

  return res.data;
}

export async function updateConfiguration(
  localUser: StoredUser,
  key: string,
  value: string
): Promise<ConfigItem> {
  const { userId } = localUser;
  if (!userId) throw new Error("No userId");

  const res = await axios.patch(
    `${API_BASE_URL}/api/users/${userId}/config/${key}`,
    { value },
  );

  return res.data;
}

export async function bulkUpdateConfiguration(
  localUser: StoredUser,
  updates: { updates: Record<string, string> }
): Promise<Record<string, ConfigItem>> {
  const { userId } = localUser;
  if (!userId) throw new Error("No userId");

  const res = await axios.post(
    `${API_BASE_URL}/api/users/${userId}/config/bulk`,
    updates,
  );

  return res.data;
}