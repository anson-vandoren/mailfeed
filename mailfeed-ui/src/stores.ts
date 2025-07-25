import { localStorageStore } from "@skeletonlabs/skeleton";
import type { Writable } from "svelte/store";

export type StoredUser = {
  email?: string;
  token?: string;
  refresh?: string;
  userId?: number;
};
export const user: Writable<StoredUser> = localStorageStore("user", {});