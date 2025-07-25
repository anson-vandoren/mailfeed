import { browser } from '$app/environment';
import { dev } from '$app/environment';

// Configuration for API endpoints
const getApiBaseUrl = (): string => {
  if (browser && dev) {
    // In development, use the dev server (which proxies to backend)
    return window.location.origin;
  }
  
  if (browser) {
    // In production browser, use same origin as the page
    return window.location.origin;
  }
  
  // During SSR/build, use environment variable or default
  return import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';
};

export const API_BASE_URL = getApiBaseUrl();