#!/usr/bin/env python3
"""Extract text from a .docx file using only stdlib (zipfile + XML parsing)."""
import sys
import zipfile
import xml.etree.ElementTree as ET

NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "w14": "http://schemas.microsoft.com/office/word/2010/wordml",
}


def extract(path: str) -> str:
    out = []
    with zipfile.ZipFile(path) as z:
        # Try the main document; fall back to any word/*.xml containing w:p
        names = [n for n in z.namelist() if n.startswith("word/") and n.endswith(".xml")]
        names.sort()
        for name in names:
            if "document" not in name and "header" not in name and "footer" not in name:
                continue
            try:
                root = ET.fromstring(z.read(name))
            except ET.ParseError:
                continue
            for p in root.iter("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}p"):
                texts = []
                for t in p.iter("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}t"):
                    texts.append(t.text or "")
                line = "".join(texts)
                # Mark headings-ish or page breaks
                ppr = p.find("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pPr")
                if ppr is not None and ppr.find("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pageBreakBefore") is not None:
                    out.append("\n=== PAGE BREAK ===\n")
                if line.strip():
                    out.append(line)
    return "\n".join(out)


if __name__ == "__main__":
    src = sys.argv[1]
    dst = sys.argv[2] if len(sys.argv) > 2 else None
    text = extract(src)
    if dst:
        with open(dst, "w", encoding="utf-8") as f:
            f.write(text)
        print(f"Wrote {len(text)} chars to {dst}")
    else:
        print(text)
