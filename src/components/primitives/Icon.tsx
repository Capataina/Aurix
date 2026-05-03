import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const baseProps: SVGProps<SVGSVGElement> = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": true,
};

export function GripIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <circle cx={6} cy={4} r={0.8} fill="currentColor" stroke="none" />
      <circle cx={10} cy={4} r={0.8} fill="currentColor" stroke="none" />
      <circle cx={6} cy={8} r={0.8} fill="currentColor" stroke="none" />
      <circle cx={10} cy={8} r={0.8} fill="currentColor" stroke="none" />
      <circle cx={6} cy={12} r={0.8} fill="currentColor" stroke="none" />
      <circle cx={10} cy={12} r={0.8} fill="currentColor" stroke="none" />
    </svg>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </svg>
  );
}

export function PanelRightIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <rect x={2} y={3} width={12} height={10} rx={1.5} />
      <path d="M10 3v10" />
    </svg>
  );
}

export function RefreshIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M2.5 8a5.5 5.5 0 019.4-3.9L13.5 5.6" />
      <path d="M13.5 8a5.5 5.5 0 01-9.4 3.9L2.5 10.4" />
      <path d="M11 2.5L13.5 5l-2.5 2.5" />
      <path d="M5 13.5L2.5 11 5 8.5" />
    </svg>
  );
}

export function PauseIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <rect x={4.5} y={3} width={2.5} height={10} rx={0.5} fill="currentColor" stroke="none" />
      <rect x={9} y={3} width={2.5} height={10} rx={0.5} fill="currentColor" stroke="none" />
    </svg>
  );
}

export function ChartIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M2 12l3-4 3 2 5-6" />
      <path d="M2 14h12" />
    </svg>
  );
}

export function GridIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <rect x={2} y={2} width={5} height={5} rx={1} />
      <rect x={9} y={2} width={5} height={5} rx={1} />
      <rect x={2} y={9} width={5} height={5} rx={1} />
      <rect x={9} y={9} width={5} height={5} rx={1} />
    </svg>
  );
}

export function FlameIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M8 2c0 3-3 4-3 7a3 3 0 006 0c0-2-1-3-1-4 1 0 2 1 2 3a4 4 0 11-8 0c0-3 4-4 4-6z" />
    </svg>
  );
}

export function PulseIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M2 8h3l1.5-3 3 6 1.5-3H14" />
    </svg>
  );
}

export function StatsIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M3 13V8M7 13V4M11 13V10" />
      <path d="M2 13h12" />
    </svg>
  );
}

export function PriceTagIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M8.5 2H13a1 1 0 011 1v4.5L7 14.5l-5.5-5.5L8.5 2z" />
      <circle cx={11} cy={5} r={0.8} fill="currentColor" stroke="none" />
    </svg>
  );
}

export function LinkIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M7 9a3 3 0 004.2 0l2-2a3 3 0 10-4.2-4.2L7.5 4" />
      <path d="M9 7a3 3 0 00-4.2 0l-2 2a3 3 0 104.2 4.2L8.5 12" />
    </svg>
  );
}

export function NewsIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <rect x={2} y={3} width={12} height={10} rx={1.5} />
      <path d="M5 6h6M5 8.5h6M5 11h4" />
    </svg>
  );
}

export function ResetIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <path d="M2.5 5.5a5.5 5.5 0 109.5-1.5" />
      <path d="M2.5 2v3.5h3.5" />
    </svg>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <svg {...baseProps} {...props}>
      <circle cx={8} cy={8} r={2} />
      <path d="M8 1.5v2M8 12.5v2M14.5 8h-2M3.5 8h-2M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4M12.6 12.6l-1.4-1.4M4.8 4.8L3.4 3.4" />
    </svg>
  );
}
