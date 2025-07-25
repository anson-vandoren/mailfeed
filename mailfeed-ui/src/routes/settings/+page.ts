import { browser } from '$app/environment';

export const prerender = false;
export const ssr = false;

/** @type {import('./$types').PageLoad} */
export async function load() {
  if (!browser) {
    return {};
  }
  
  // Data will be loaded client-side in the component
  return {};
}