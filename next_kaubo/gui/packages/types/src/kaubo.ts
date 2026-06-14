export type AppStatus = "idle" | "compiling" | "ready" | "running";

export interface KauboCompileResult {
  success: boolean;
  output: string;
  errors: string[];
}

export interface KauboError {
  kind: string;
  message: string;
  sourceLine?: number;
}
