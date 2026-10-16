import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { isGoogleApp, isGoogleSystemComponent } from "../src/googleApps.mjs";
import { isPreinstalled } from "../src/preinstalled.mjs";

describe("isGoogleApp", () => {
    it("matches com.google.android.*", () => {
    assert.equal(isGoogleApp("com.google.android.apps.maps"), true);
    assert.equal(isGoogleApp("com.google.android.youtube"), true);
    assert.equal(isGoogleApp("com.google.android.gm"), true);
  });

  it("matches Chrome exactly", () => {
    assert.equal(isGoogleApp("com.android.chrome"), true);
  });

  it("matches Lens", () => {
    assert.equal(isGoogleApp("com.google.ar.lens"), true);
    assert.equal(isGoogleSystemComponent("com.google.ar.lens"), false);
  });

  it("marks only the two preinstalled exceptions", () => {
    assert.equal(isPreinstalled("com.oplus.games"), true);
    assert.equal(isPreinstalled("com.netflix.partner.activation"), true);
    assert.equal(isPreinstalled("com.facebook.appmanager"), false);
    assert.equal(isPreinstalled("com.coloros.weather.service"), false);
  });

  it("rejects other com.google.* without .android", () => {
    assert.equal(isGoogleApp("com.google.ar.core"), false);
    assert.equal(isGoogleApp("com.google.protobuf"), false);
  });

  it("keeps system modules out of the GApps list", () => {
    assert.equal(isGoogleSystemComponent("com.google.android.ext.services"), true);
    assert.equal(isGoogleSystemComponent("com.google.android.networkstack.tethering"), true);
    assert.equal(isGoogleSystemComponent("com.google.android.gms"), true);
    assert.equal(isGoogleApp("com.google.android.ext.services"), false);
    assert.equal(isGoogleApp("com.google.android.networkstack.tethering"), false);
    assert.equal(isGoogleApp("com.google.android.gms"), false);
    assert.equal(isGoogleApp("com.google.android.apps.youtube.music"), true);
    assert.equal(isGoogleSystemComponent("com.google.android.gm"), false);
  });

  it("rejects unrelated packages", () => {
    assert.equal(isGoogleApp("com.samsung.android.app"), false);
    assert.equal(isGoogleApp("com.android.vending"), false);
    assert.equal(isGoogleApp("com.google"), false);
    assert.equal(isGoogleApp("com.google.android"), false);
  });

  it("rejects non-string package names", () => {
    assert.equal(isGoogleApp(null), false);
    assert.equal(isGoogleApp(undefined), false);
    assert.equal(isGoogleApp(42), false);
  });
});
