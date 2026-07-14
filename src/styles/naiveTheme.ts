import type { GlobalThemeOverrides } from "naive-ui";

const fontFamily =
  '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const fontFamilyMono =
  '"SF Mono", "JetBrains Mono", "IBM Plex Mono", ui-monospace, monospace';

export const darkThemeOverrides: GlobalThemeOverrides = {
  common: {
    bodyColor: "#1f2329",
    cardColor: "#252a31",
    modalColor: "#252a31",
    popoverColor: "#292f37",
    tableColor: "#252a31",
    textColorBase: "#f3f4f6",
    textColor1: "#f3f4f6",
    textColor2: "#b8bec8",
    textColor3: "#858d99",
    borderColor: "rgba(255, 255, 255, 0.1)",
    primaryColor: "#0a84ff",
    primaryColorHover: "#2997ff",
    primaryColorPressed: "#0071e3",
    successColor: "#30d158",
    warningColor: "#ff9f0a",
    errorColor: "#ff453a",
    fontFamily,
    fontFamilyMono,
    borderRadius: "8px",
  },
  Button: {
    heightSmall: "28px",
    borderRadiusSmall: "7px",
    fontWeight: "500",
  },
  DataTable: {
    thColor: "#2a3038",
    tdColor: "#252a31",
    tdColorHover: "#2b323b",
    borderColor: "rgba(255, 255, 255, 0.075)",
    thTextColor: "#9aa3af",
    tdTextColor: "#e8eaed",
  },
  Dropdown: {
    color: "#292f37",
    optionTextColor: "#f3f4f6",
    optionColorHover: "rgba(255, 255, 255, 0.07)",
  },
  Input: {
    color: "#2a3038",
    colorFocus: "#2a3038",
    border: "1px solid rgba(255, 255, 255, 0.1)",
    borderHover: "1px solid rgba(10, 132, 255, 0.45)",
    borderFocus: "1px solid rgba(10, 132, 255, 0.66)",
    boxShadowFocus: "0 0 0 3px rgba(10, 132, 255, 0.16)",
  },
  Popconfirm: {
    color: "#292f37",
  },
  Switch: {
    railColorActive: "#0a84ff",
    railColorActiveHover: "#2997ff",
    loadingColor: "#0a84ff",
  },
};

export const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    bodyColor: "#f2f2f4",
    cardColor: "#ffffff",
    modalColor: "#ffffff",
    popoverColor: "#ffffff",
    tableColor: "#ffffff",
    textColorBase: "#1d1d1f",
    textColor1: "#1d1d1f",
    textColor2: "#51565d",
    textColor3: "#777d86",
    borderColor: "rgba(60, 60, 67, 0.13)",
    primaryColor: "#0071e3",
    primaryColorHover: "#0a84ff",
    primaryColorPressed: "#005bbf",
    successColor: "#248a3d",
    warningColor: "#bf6a02",
    errorColor: "#d70015",
    fontFamily,
    fontFamilyMono,
    borderRadius: "8px",
  },
  Button: {
    heightSmall: "28px",
    borderRadiusSmall: "7px",
    fontWeight: "500",
  },
  DataTable: {
    thColor: "#f3f4f6",
    tdColor: "#ffffff",
    tdColorHover: "#f6f7f9",
    borderColor: "rgba(60, 60, 67, 0.11)",
    thTextColor: "#6b7280",
    tdTextColor: "#1f2933",
  },
  Dropdown: {
    color: "#ffffff",
    optionTextColor: "#1d1d1f",
    optionColorHover: "rgba(0, 0, 0, 0.045)",
  },
  Input: {
    color: "#ffffff",
    colorFocus: "#ffffff",
    border: "1px solid rgba(60, 60, 67, 0.14)",
    borderHover: "1px solid rgba(0, 113, 227, 0.34)",
    borderFocus: "1px solid rgba(0, 113, 227, 0.58)",
    boxShadowFocus: "0 0 0 3px rgba(0, 113, 227, 0.14)",
  },
  Popconfirm: {
    color: "#ffffff",
  },
};
