/**
 * Minimal, safe markdown renderer for assistant output.
 *
 * Security requirement (docs/local-security-boundary.md section 4): model
 * output is never rendered as raw HTML. This renderer builds React elements
 * directly -- there is no dangerouslySetInnerHTML anywhere -- so HTML in the
 * model output displays as literal text.
 *
 * Supports: fenced code blocks, inline code, bold, italic, headings,
 * unordered and ordered lists, paragraphs.
 */
import React from "react";

function renderInline(text: string, keyBase: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  // Tokenize inline code first so formatting inside code is preserved.
  const parts = text.split(/(`[^`]+`)/g);
  parts.forEach((part, i) => {
    const key = `${keyBase}-${i}`;
    if (part.startsWith("`") && part.endsWith("`") && part.length > 2) {
      nodes.push(
        <code key={key} className="rounded bg-brand-hover px-1.5 py-0.5 font-mono text-[0.85em] text-accent-secondary">
          {part.slice(1, -1)}
        </code>,
      );
      return;
    }
    // Bold and italic within plain segments.
    const segments = part.split(/(\*\*[^*]+\*\*|\*[^*]+\*)/g);
    segments.forEach((seg, j) => {
      const k = `${key}-${j}`;
      if (seg.startsWith("**") && seg.endsWith("**") && seg.length > 4) {
        nodes.push(<strong key={k}>{seg.slice(2, -2)}</strong>);
      } else if (seg.startsWith("*") && seg.endsWith("*") && seg.length > 2) {
        nodes.push(<em key={k}>{seg.slice(1, -1)}</em>);
      } else if (seg.length > 0) {
        nodes.push(<React.Fragment key={k}>{seg}</React.Fragment>);
      }
    });
  });
  return nodes;
}

function CodeBlock({ code }: { code: string }) {
  const [copied, setCopied] = React.useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="group relative my-2">
      <button
        onClick={() => void handleCopy()}
        className="absolute top-2 right-2 hidden rounded border border-brand-border bg-brand-hover px-2 py-1 text-xs text-brand-textMuted transition-colors hover:text-zinc-100 group-hover:block"
        title="Copy code"
      >
        {copied ? "Copied!" : "Copy"}
      </button>
      <pre className="overflow-x-auto rounded-lg border border-brand-border bg-brand-card p-3 font-mono text-[0.8rem] leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  );
}

export function Markdown({ text }: { text: string }) {
  const blocks: React.ReactNode[] = [];
  const lines = text.split("\n");
  let i = 0;
  let blockIdx = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code block
    if (line.trimStart().startsWith("```")) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trimStart().startsWith("```")) {
        code.push(lines[i]);
        i++;
      }
      i++; // closing fence (or EOF)
      blocks.push(
        <CodeBlock key={blockIdx++} code={code.join("\n")} />
      );
      continue;
    }

    // Heading
    const heading = /^(#{1,4})\s+(.*)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const sizes = ["text-lg", "text-base", "text-sm", "text-sm"];
      blocks.push(
        <p key={blockIdx++} className={`mt-3 mb-1 font-semibold ${sizes[level - 1]}`}>
          {renderInline(heading[2], `h${blockIdx}`)}
        </p>,
      );
      i++;
      continue;
    }

    // List (unordered or ordered)
    const isListLine = (l: string) => /^\s*([-*]|\d+\.)\s+/.test(l);
    if (isListLine(line)) {
      const items: string[] = [];
      const ordered = /^\s*\d+\./.test(line);
      while (i < lines.length && isListLine(lines[i])) {
        items.push(lines[i].replace(/^\s*([-*]|\d+\.)\s+/, ""));
        i++;
      }
      const cls = "my-1 space-y-0.5 pl-5";
      blocks.push(
        ordered ? (
          <ol key={blockIdx++} className={`list-decimal ${cls}`}>
            {items.map((it, j) => (
              <li key={j}>{renderInline(it, `li${blockIdx}-${j}`)}</li>
            ))}
          </ol>
        ) : (
          <ul key={blockIdx++} className={`list-disc ${cls}`}>
            {items.map((it, j) => (
              <li key={j}>{renderInline(it, `li${blockIdx}-${j}`)}</li>
            ))}
          </ul>
        ),
      );
      continue;
    }

    // Blank line
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Paragraph: consume consecutive non-empty, non-special lines
    const para: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].trimStart().startsWith("```") &&
      !/^(#{1,4})\s+/.test(lines[i]) &&
      !isListLine(lines[i])
    ) {
      para.push(lines[i]);
      i++;
    }
    blocks.push(
      <p key={blockIdx++} className="my-1 whitespace-pre-wrap leading-relaxed">
        {renderInline(para.join("\n"), `p${blockIdx}`)}
      </p>,
    );
  }

  return <div className="text-sm">{blocks}</div>;
}
