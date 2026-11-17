export type PortContainer = {
  name: string;
  image: string;
  project: string;
  service: string;
  containerPort: number | null;
};

export type PortProcess = {
  pid: number;
  processName: string;
  user: string;
  protocol: string;
  port: number;
  state: string;
  hosts: string[];
  hostSummary: string;
  ipVersions: string[];
  command: string;
  cwd: string | null;
  startedAt: string | null;
  startedAtTs: number | null;
  container: PortContainer | null;
  dockerProxy: boolean;
};

export type PortListResponse = {
  currentUser: string;
  items: PortProcess[];
  dockerAvailable: boolean;
};

export type KillResult = {
  pid: number;
  signal: string;
  success: boolean;
  message: string;
};

export type ReboundCheckResult = {
  port: number;
  occupied: boolean;
  rebound: boolean;
  sameProcessName: boolean;
  pid: number | null;
  processName: string | null;
  command: string | null;
  message: string | null;
};

export type SortKey = "recent" | "port" | "process";

export type SortOrder = "ascend" | "descend";
