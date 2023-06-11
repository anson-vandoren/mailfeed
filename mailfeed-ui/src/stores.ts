import { localStorageStore } from "@skeletonlabs/skeleton";
import type { Writable } from "svelte/store";

type StoredUser = {
  email?: string;
  token?: string;
  refresh?: string;
};
export const user: Writable<StoredUser> = localStorageStore("user", {});