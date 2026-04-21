import { invoke } from "@tauri-apps/api/core";

export interface RemoteHost {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  password: string;
  created_at: number;
}

export interface ActiveRemoteInfo {
  host_id: string;
  name: string;
  host: string;
  username: string;
  remote_home: string;
}

export interface CreateRemoteHostRequest {
  name: string;
  host: string;
  port?: number;
  username: string;
  password: string;
}

export interface UpdateRemoteHostRequest {
  id: string;
  name: string;
  host: string;
  port?: number;
  username: string;
  password: string;
}

export const remoteHostApi = {
  async list(): Promise<RemoteHost[]> {
    return await invoke("list_remote_hosts");
  },

  async create(req: CreateRemoteHostRequest): Promise<RemoteHost> {
    return await invoke("create_remote_host", { req });
  },

  async update(req: UpdateRemoteHostRequest): Promise<RemoteHost> {
    return await invoke("update_remote_host", { req });
  },

  async delete(id: string): Promise<void> {
    return await invoke("delete_remote_host", { id });
  },

  async connect(id: string): Promise<ActiveRemoteInfo> {
    return await invoke("connect_remote_host", { id });
  },

  async disconnect(): Promise<void> {
    return await invoke("disconnect_remote_host");
  },

  async getActive(): Promise<ActiveRemoteInfo | null> {
    return await invoke("get_active_remote_host");
  },

  async testConnection(
    host: string,
    port: number,
    username: string,
    password: string
  ): Promise<string> {
    return await invoke("test_remote_connection", { host, port, username, password });
  },
};
