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
};

export type PortListResponse = {
  currentUser: string;
  items: PortProcess[];
};

export type KillResult = {
  pid: number;
  signal: string;
  success: boolean;
  message: string;
};

export type SortKey = "recent" | "port" | "process";

export type SortOrder = "ascend" | "descend";

export type PortTagTone = "framework" | "service" | "infra";

export type PortTag = {
  label: string;
  tone: PortTagTone;
};
