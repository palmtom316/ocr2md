import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { LayoutShell } from "./LayoutShell";

describe("LayoutShell", () => {
  it("renders dual-pane shell with left queue and right workspace", () => {
    render(<LayoutShell />);
    expect(screen.getByTestId("queue-pane")).toBeInTheDocument();
    expect(screen.getByTestId("workspace-pane")).toBeInTheDocument();
  });

  it("shows icon-supported navigation and pane headers", () => {
    render(<LayoutShell />);
    expect(screen.getByLabelText("Queue navigation icon")).toBeInTheDocument();
    expect(screen.getByLabelText("Workspace pane icon")).toBeInTheDocument();
    expect(screen.getByLabelText("Settings pane icon")).toBeInTheDocument();
  });

  it("renders api profile settings form", () => {
    render(<LayoutShell />);
    expect(screen.getByLabelText("Master passphrase")).toBeInTheDocument();
    expect(screen.getByLabelText("Provider")).toBeInTheDocument();
    expect(screen.getByLabelText("Base URL")).toBeInTheDocument();
    expect(screen.getByLabelText("API Key")).toBeInTheDocument();
    expect(screen.getByLabelText("Model")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Load Profiles" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save Profiles" })).toBeInTheDocument();
  });
});
