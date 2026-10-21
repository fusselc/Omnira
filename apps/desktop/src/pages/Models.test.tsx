import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { ModelUseButton } from "./Models";

describe("ModelUseButton", () => {
  it("renders a disabled In use control for the active model and does not call start", () => {
    const onUse = vi.fn();
    const html = renderToStaticMarkup(
      <ModelUseButton active disabled={false} onUse={onUse} />,
    );
    expect(html).toContain("In use");
    expect(html).toContain("disabled");
    expect(html).not.toContain("Restart");
    expect(html).not.toContain("Use");
    expect(onUse).not.toHaveBeenCalled();
  });

  it("renders Use for a model that is not running", () => {
    const html = renderToStaticMarkup(
      <ModelUseButton active={false} disabled={false} onUse={() => {}} />,
    );
    expect(html).toContain("Use");
    expect(html).not.toContain("Restart");
    expect(html).not.toContain("In use");
  });
});
