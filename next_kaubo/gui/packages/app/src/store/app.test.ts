import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { mockDoCompile, mockDoRun, mockDoDiagnose, mockDoFormat, mockDoLspOnChange, mockSetKauboDiagnostics } =
  vi.hoisted(() => ({
    mockDoCompile: vi.fn(),
    mockDoRun: vi.fn(),
    mockDoDiagnose: vi.fn().mockReturnValue("[]"),
    mockDoFormat: vi.fn(),
    mockDoLspOnChange: vi.fn(),
    mockSetKauboDiagnostics: vi.fn(),
  }));

vi.mock("../hooks/useKaubo", () => ({
  useKaubo: () => ({
    doCompile: mockDoCompile,
    doRun: mockDoRun,
    doDiagnose: mockDoDiagnose,
    doFormat: mockDoFormat,
    doLspOnChange: mockDoLspOnChange,
    loading: () => false,
  }),
}));

vi.mock("../editor/kauboLang", async () => {
  const actual = await vi.importActual<typeof import("../editor/kauboLang")>(
    "../editor/kauboLang",
  );
  return {
    ...actual,
    setKauboDiagnostics: mockSetKauboDiagnostics,
  };
});

import { createKauboStore } from "./app";

describe("createKauboStore", () => {
  let store: ReturnType<typeof createKauboStore>;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    store = createKauboStore();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("initial state is idle", () => {
    expect(store.status()).toBe("idle");
  });

  it("initial output is empty", () => {
    expect(store.output()).toBe("");
  });

  it("initial error is null", () => {
    expect(store.error()).toBeNull();
  });

  it("has default code", () => {
    expect(store.code()).toContain("Hello, World!");
  });

  it("setCode updates code signal", () => {
    store.setCode("var x = 1;");
    expect(store.code()).toBe("var x = 1;");
  });

  it("setCode calls diagnose and lsp_on_change immediately", () => {
    store.setCode("var x");
    expect(mockDoDiagnose).toHaveBeenCalledWith("var x");
    expect(mockDoLspOnChange).toHaveBeenCalledWith("var x");
  });

  describe("compile", () => {
    it("sets status to ready on success and clears diagnostics", () => {
      mockDoCompile.mockReturnValue(42);
      store.compile();

      expect(store.status()).toBe("ready");
      expect(store.output()).toContain("Compiled: 42");
      expect(mockSetKauboDiagnostics).toHaveBeenCalledWith(null);
    });

    it("sets error and runs diagnose on failure", () => {
      mockDoCompile.mockImplementation(() => {
        throw new Error("parse error at line 1");
      });
      mockDoDiagnose.mockReturnValue(
        '[{"severity":"error","from":0,"to":3,"message":"bad"}]',
      );

      store.compile();

      expect(store.status()).toBe("idle");
      expect(store.error()).toBe("parse error at line 1");
      expect(mockDoDiagnose).toHaveBeenCalled();
      expect(mockSetKauboDiagnostics).toHaveBeenCalled();
    });

    it("sets error when WASM is not loaded", () => {
      mockDoCompile.mockReturnValue(null);
      store.compile();

      expect(store.status()).toBe("idle");
      expect(store.error()).toBe("WASM not loaded yet");
    });
  });

  describe("run", () => {
    it("compiles, runs, and clears diagnostics on success", async () => {
      mockDoCompile.mockReturnValue(42);
      mockDoRun.mockReturnValue("Hello\n");

      store.run();
      await vi.advanceTimersByTimeAsync(20);

      expect(mockDoCompile).toHaveBeenCalled();
      expect(mockDoRun).toHaveBeenCalled();
      expect(store.output()).toContain("Hello");
      expect(store.status()).toBe("ready");
      expect(mockSetKauboDiagnostics).toHaveBeenCalledWith(null);
    });

    it("sets error when compile returns null", async () => {
      mockDoCompile.mockReturnValue(null);

      store.run();
      await vi.advanceTimersByTimeAsync(20);

      expect(store.error()).toBe("WASM not loaded yet");
    });

    it("sets error when run returns null", async () => {
      mockDoCompile.mockReturnValue(42);
      mockDoRun.mockReturnValue(null);

      store.run();
      await vi.advanceTimersByTimeAsync(20);

      expect(store.error()).toBe("WASM not loaded yet");
    });

    it("runs diagnose on compile failure during run", async () => {
      mockDoCompile.mockImplementation(() => {
        throw new Error("bad");
      });

      store.run();
      await vi.advanceTimersByTimeAsync(20);

      expect(mockDoDiagnose).toHaveBeenCalled();
    });
  });

  describe("clearError", () => {
    it("clears error and diagnostics", () => {
      mockDoCompile.mockReturnValue(null);
      store.compile();
      expect(store.error()).toBeTruthy();

      store.clearError();

      expect(store.error()).toBeNull();
      expect(mockSetKauboDiagnostics).toHaveBeenCalledWith(null);
    });
  });

  describe("clearOutput", () => {
    it("resets output to empty", () => {
      mockDoCompile.mockReturnValue(42);
      store.compile();
      expect(store.output()).toContain("Compiled");

      store.clearOutput();
      expect(store.output()).toBe("");
    });
  });

  describe("theme", () => {
    it("defaults to material-dark", () => {
      expect(store.theme()).toBe("material-dark");
    });

    it("setTheme updates signal", () => {
      store.setTheme("nord");
      expect(store.theme()).toBe("nord");
    });
  });

  describe("examplesExpanded", () => {
    it("defaults to true", () => {
      expect(store.examplesExpanded()).toBe(true);
    });

    it("toggleExamples flips the value", () => {
      store.toggleExamples();
      expect(store.examplesExpanded()).toBe(false);
      store.toggleExamples();
      expect(store.examplesExpanded()).toBe(true);
    });
  });

  describe("loadExample", () => {
    it("loads example code and sets active ID", () => {
      store.loadExample({
        id: "hello",
        name: "Test",
        description: "desc",
        code: "print(1);",
        tags: [],
      });
      expect(store.activeExample()).toBe("hello");
      expect(store.code()).toBe("print(1);");
      expect(store.error()).toBeNull();
      expect(mockSetKauboDiagnostics).toHaveBeenCalledWith(null);
    });

    it("clears output when loading example", () => {
      mockDoCompile.mockReturnValue(42);
      store.compile();
      expect(store.output()).toContain("Compiled");

      store.loadExample({
        id: "hello",
        name: "Test",
        description: "desc",
        code: "x",
        tags: [],
      });
      expect(store.output()).toBe("");
    });
  });

  describe("activeExample", () => {
    it("starts null", () => {
      expect(store.activeExample()).toBeNull();
    });

    it("is cleared when user types", () => {
      store.loadExample({
        id: "hello",
        name: "Test",
        description: "desc",
        code: "print(1);",
        tags: [],
      });
      expect(store.activeExample()).toBe("hello");

      store.setCode("var x = 1;");
      expect(store.activeExample()).toBeNull();
    });
  });

  describe("tabSize", () => {
    it("defaults to 4", () => {
      expect(store.tabSize()).toBe(4);
    });

    it("setTabSize updates signal", () => {
      store.setTabSize(2);
      expect(store.tabSize()).toBe(2);
    });
  });

  describe("fontSize", () => {
    it("defaults to 14", () => {
      expect(store.fontSize()).toBe(14);
    });

    it("setFontSize updates signal", () => {
      store.setFontSize(16);
      expect(store.fontSize()).toBe(16);
    });
  });

  describe("settingsOpen", () => {
    it("defaults to false", () => {
      expect(store.settingsOpen()).toBe(false);
    });

    it("toggleSettings flips the value", () => {
      store.toggleSettings();
      expect(store.settingsOpen()).toBe(true);
      store.toggleSettings();
      expect(store.settingsOpen()).toBe(false);
    });
  });

  describe("resetSettings", () => {
    it("resets to defaults", () => {
      store.setTheme("nord");
      store.setTabSize(2);
      store.setFontSize(16);
      store.resetSettings();

      expect(store.theme()).toBe("material-dark");
      expect(store.tabSize()).toBe(4);
      expect(store.fontSize()).toBe(14);
    });
  });
});
