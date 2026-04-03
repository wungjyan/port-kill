export type PortHint = {
  port: number;
  labels: string[];
  tone?: "framework" | "service" | "infra";
};

export const defaultPortHints: PortHint[] = [
  { port: 3000, labels: ["Next.js", "NestJS", "React Dev"], tone: "framework" },
  { port: 3001, labels: ["React Dev", "Webpack Dev"], tone: "framework" },
  { port: 4173, labels: ["Vite Preview"], tone: "framework" },
  { port: 4200, labels: ["Angular", "Nx"], tone: "framework" },
  { port: 4321, labels: ["Astro"], tone: "framework" },
  { port: 5000, labels: ["Flask", "Django Dev", "Rails Puma"], tone: "framework" },
  { port: 5173, labels: ["Vite", "Vue", "React", "Svelte"], tone: "framework" },
  { port: 5432, labels: ["PostgreSQL"], tone: "service" },
  { port: 5500, labels: ["Live Server"], tone: "framework" },
  { port: 6379, labels: ["Redis"], tone: "infra" },
  { port: 8000, labels: ["Python HTTP", "Django", "uvicorn"], tone: "framework" },
  { port: 8080, labels: ["Spring Boot", "Proxy", "HTTP Alt"], tone: "service" },
  { port: 8081, labels: ["Webpack", "Dev Server"], tone: "framework" },
  { port: 8787, labels: ["Wrangler", "Cloudflare Dev"], tone: "framework" },
  { port: 8888, labels: ["Jupyter", "Local Lab"], tone: "service" },
  { port: 9229, labels: ["Node Inspect"], tone: "service" },
  { port: 27017, labels: ["MongoDB"], tone: "infra" },
];

export function loadPortHints(): PortHint[] {
  if (typeof window === "undefined") {
    return defaultPortHints;
  }

  const rawValue = window.localStorage.getItem("port-kill.port-hints");
  if (!rawValue) {
    return defaultPortHints;
  }

  try {
    const parsed = JSON.parse(rawValue) as PortHint[];
    if (!Array.isArray(parsed)) {
      return defaultPortHints;
    }

    return parsed.filter(
      (item) => typeof item?.port === "number" && Array.isArray(item?.labels),
    );
  } catch {
    return defaultPortHints;
  }
}
