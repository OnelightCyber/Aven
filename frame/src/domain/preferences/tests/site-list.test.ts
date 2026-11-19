import { describe, expect, it } from "vitest";
import { covers, siteOf, sitesOf, withSite, withoutSite } from "../site-list";

describe("a list of sites set apart", () => {
  it("names a web page's site without its www", () => {
    expect(siteOf("https://www.Example.com/a?b")).toBe("example.com");
    expect(siteOf("http://shop.example.com:8080/")).toBe("shop.example.com");
    expect(siteOf("aven://settings")).toBe("");
    expect(siteOf("not an address")).toBe("");
    expect(siteOf(null)).toBe("");
  });

  it("covers a site and the names under it, and nothing that only ends alike", () => {
    const stored = "example.com\nnews.example";
    expect(covers(stored, "example.com")).toBe(true);
    expect(covers(stored, "shop.example.com")).toBe(true);
    expect(covers(stored, "notexample.com")).toBe(false);
    expect(covers(stored, "")).toBe(false);
    expect(covers("", "example.com")).toBe(false);
  });

  it("adds a site once and takes away whatever covers it", () => {
    expect(withSite("", "example.com")).toBe("example.com");
    expect(withSite("example.com", "example.com")).toBe("example.com");
    expect(withSite("example.com", "shop.example.com")).toBe("example.com");
    expect(withSite("example.com", "news.example")).toBe("example.com\nnews.example");
    expect(withoutSite("example.com\nnews.example", "shop.example.com")).toBe("news.example");
    expect(withoutSite("example.com", "other.test")).toBe("example.com");
    expect(sitesOf(" example.com \n\n news.example ")).toEqual(["example.com", "news.example"]);
  });
});
