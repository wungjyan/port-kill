import { defaultPortHints, loadPortHints } from "./portHints";
import type { PortProcess, PortTag, PortTagTone } from "./types";

const monthMap: Record<string, string> = {
  Jan: "01",
  Feb: "02",
  Mar: "03",
  Apr: "04",
  May: "05",
  Jun: "06",
  Jul: "07",
  Aug: "08",
  Sep: "09",
  Oct: "10",
  Nov: "11",
  Dec: "12",
};

const toneMap: Record<PortTagTone, "info" | "success" | "warning"> = {
  framework: "info",
  service: "success",
  infra: "warning",
};

const portHints = typeof window === "undefined" ? defaultPortHints : loadPortHints();

export function resolvePortTags(item: PortProcess): PortTag[] {
  const matched = portHints.find((hint) => hint.port === item.port);
  if (!matched) {
    return [];
  }

  return matched.labels.map((label) => ({
    label,
    tone: matched.tone ?? "service",
  }));
}

export function tagTypeForTone(tone: PortTagTone): "info" | "success" | "warning" {
  return toneMap[tone];
}

export function formatStartedAtShort(value: string | null): string {
  if (!value) {
    return "未知";
  }

  const parts = value.split(/\s+/);
  if (parts.length < 5) {
    return value;
  }

  const month = monthMap[parts[1]];
  const day = parts[2].padStart(2, "0");
  const time = parts[3].slice(0, 5);

  if (!month) {
    return value;
  }

  return `${month}-${day} ${time}`;
}

export function formatStartedAtFull(value: string | null): string {
  return value ?? "未知";
}

export function formatAddressSummary(item: PortProcess): string {
  if (item.hosts.length === 0) {
    return "-";
  }

  return item.hosts.join(" / ");
}

export function formatIpSummary(item: PortProcess): string {
  if (item.ipVersions.length > 1) {
    return "IPv4 + IPv6";
  }

  return item.ipVersions[0] ?? "TCP";
}

export function getRowKey(item: PortProcess): string {
  return `${item.pid}:${item.port}:${item.protocol}`;
}
