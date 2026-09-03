/**
 * Contract tests for the custom markdown renderer (docs/local-security-boundary.md
 * section 4): model output must never execute as HTML, and the supported
 * fenced/inline code and formatting must render as expected React elements.
 *
 * Uses `renderToStaticMarkup` from `react-dom/server` (already a project
 * dependency) instead of a UI test framework -- it renders the real React
 * tree the component produces without needing jsdom or a browser.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Markdown } from "./markdown";

function render(text: string): string {
  return renderToStaticMarkup(<Markdown text={text} />);
}

describe("Markdown", () => {
  it("never renders HTML in model output as live markup", () => {
    const html = render("<img src=x onerror=alert(1)> <b>not bold via HTML</b>");
    // The literal tags must be escaped text, not real <img>/<b> elements.
    expect(html).not.toContain("<img");
    expect(html).not.toContain("<b>");
    expect(html).toContain("&lt;img");
    expect(html).toContain("&lt;b&gt;not bold via HTML&lt;/b&gt;");
  });

  it("never renders a script tag as an executable element", () => {
    const html = render("<script>alert('xss')</script>");
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });

  it("renders fenced code blocks as <pre><code>", () => {
    const html = render("```\nconst x = 1;\nconsole.log(x);\n```");
    expect(html).toContain("<pre");
    expect(html).toContain("<code>const x = 1;\nconsole.log(x);</code>");
  });

  it("renders inline code as <code> without interpreting HTML inside it", () => {
    const html = render("Use `<div>` for a container.");
    expect(html).toMatch(/<code[^>]*>&lt;div&gt;<\/code>/);
  });

  it("renders bold and italic as strong/em", () => {
    const html = render("**bold** and *italic*");
    expect(html).toContain("<strong>bold</strong>");
    expect(html).toContain("<em>italic</em>");
  });

  it("keeps an unterminated fence as a live code block instead of dropping content", () => {
    const html = render("```\nstill streaming, no closing fence yet");
    expect(html).toContain("<pre");
    expect(html).toContain("still streaming, no closing fence yet");
  });
});
