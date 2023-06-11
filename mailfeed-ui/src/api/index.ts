import { user } from "../stores";
import { get } from "svelte/store";
import axios from "axios";
import type { AxiosResponse } from "axios";

export function login(email: string, password: string): Promise<AxiosResponse> {
  return axios.post("http://localhost:8080/api/auth/login", { email, password });
}

export function logout(): Promise<AxiosResponse> {
  const token = get(user).token;
  return axios.post("http://localhost:8080/api/auth/logout", {}, {
    headers: {
      Authorization: `Bearer ${token}`,
    }
  });
}
