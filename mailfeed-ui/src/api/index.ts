import { user } from "../stores";
import { get } from "svelte/store";
import axios from "axios";
import type { AxiosResponse } from "axios";
import type { StoredUser } from "../stores";
import { API_BASE_URL } from "../lib/config";
import { goto } from "$app/navigation";

// Global axios interceptor for handling auth errors
axios.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401 || error.response?.status === 403) {
      console.log('Authentication failed, clearing user session');
      // Clear user session
      user.set({});
      // Redirect to login
      goto('/');
    }
    return Promise.reject(error);
  }
);

export function login(email: string, password: string): Promise<AxiosResponse> {
  return axios.post(`${API_BASE_URL}/api/auth/login`, { email, password });
}

export async function logout(): Promise<void> {
  const token = get(user).token;
  
  // Clear user state immediately
  user.set({});
  
  // Try to call backend logout endpoint (don't throw if it fails)
  if (token) {
    try {
      await axios.post(`${API_BASE_URL}/api/auth/logout`, {}, {
        headers: {
          Authorization: `Bearer ${token}`,
        }
      });
    } catch (e) {
      console.log('Backend logout failed (expected if token was already invalid)');
    }
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
  const { token, userId } = localUser;
  console.log(localUser)
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.get(`${API_BASE_URL}/api/users/${userId}`, {
    headers: {
      Authorization: `Bearer ${token}`,
    }
  });

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
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.get(`${API_BASE_URL}/api/users/${userId}/subscriptions`, {
    headers: {
      Authorization: `Bearer ${token}`,
    }
  });

  return res.data;
}

export async function createSubscription(
  localUser: StoredUser, 
  subscription: CreateSubscriptionRequest
): Promise<SubscriptionWithFeed> {
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.post(
    `${API_BASE_URL}/api/users/${userId}/subscriptions`,
    subscription,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );

  return res.data;
}

export async function updateSubscription(
  localUser: StoredUser,
  subscriptionId: number,
  updates: UpdateSubscriptionRequest
): Promise<SubscriptionWithFeed> {
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.patch(
    `${API_BASE_URL}/api/users/${userId}/subscriptions/${subscriptionId}`,
    updates,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );

  return res.data;
}

export async function deleteSubscription(
  localUser: StoredUser,
  subscriptionId: number
): Promise<void> {
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  await axios.delete(
    `${API_BASE_URL}/api/users/${userId}/subscriptions/${subscriptionId}`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );
}

export async function validateFeed(
  localUser: StoredUser,
  url: string
): Promise<ValidateFeedResponse> {
  const { token } = localUser;
  if (!token) throw new Error("No token");

  const res = await axios.post(
    `${API_BASE_URL}/api/feeds/validate`,
    { url },
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );

  return res.data;
}

export async function updateTelegramSettings(
  localUser: StoredUser,
  telegramChatId: string,
  telegramUsername?: string
): Promise<UserDetails> {
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.patch(
    `${API_BASE_URL}/api/users/${userId}`,
    {
      telegram_chat_id: telegramChatId,
      telegram_username: telegramUsername
    },
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
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
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.get(
    `${API_BASE_URL}/api/users/${userId}/test-telegram`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
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
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.get(
    `${API_BASE_URL}/api/users/${userId}/config`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );

  return res.data;
}

export async function updateConfiguration(
  localUser: StoredUser,
  key: string,
  value: string
): Promise<ConfigItem> {
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.patch(
    `${API_BASE_URL}/api/users/${userId}/config/${key}`,
    { value },
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );

  return res.data;
}

export async function bulkUpdateConfiguration(
  localUser: StoredUser,
  updates: { updates: Record<string, string> }
): Promise<Record<string, ConfigItem>> {
  const { token, userId } = localUser;
  if (!token || !userId) throw new Error("No token or userId");

  const res = await axios.post(
    `${API_BASE_URL}/api/users/${userId}/config/bulk`,
    updates,
    {
      headers: {
        Authorization: `Bearer ${token}`,
      }
    }
  );

  return res.data;
}