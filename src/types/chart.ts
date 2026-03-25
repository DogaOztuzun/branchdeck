export type ChartDataPoint = {
  cycle: number;
  score: number;
  date?: string;
};

export type PersonaLine = {
  name: string;
  color: string;
  data: ChartDataPoint[];
};

export type ChartConfig = {
  showAxes?: boolean;
  showPoints?: boolean;
  showArea?: boolean;
  showPersonas?: boolean;
  height?: number;
  width?: number;
};
