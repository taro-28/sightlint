package org.sightlint.fixtures.atlas;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Context;
import android.content.Intent;
import android.content.res.Configuration;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.Build;
import android.os.Bundle;
import android.util.DisplayMetrics;
import android.view.Display;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.view.accessibility.AccessibilityNodeInfo;
import android.widget.TextView;

import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;

public final class CaptureInstrumentation extends Instrumentation {
    private static final String RUNNER_NAME = "sightlint-atlas-android-capture";
    private static final String RUNNER_VERSION = "0.1.0";
    private Bundle arguments;

    @Override
    public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        this.arguments = arguments;
        start();
    }

    @Override
    public void onStart() {
        String scenario = arguments == null ? null : arguments.getString("scenario");
        String fixtureSourceSha256 = arguments == null
                ? null : arguments.getString("fixtureSourceSha256");
        String gradleVersion = arguments == null ? null : arguments.getString("gradleVersion");
        String adbVersion = arguments == null ? null : arguments.getString("adbVersion");
        String emulatorVersion = arguments == null ? null : arguments.getString("emulatorVersion");
        String avdName = arguments == null ? null : arguments.getString("avdName");
        if (!MainActivity.CLEAN.equals(scenario)
                && !MainActivity.OFF_CANVAS.equals(scenario)
                && !MainActivity.SCROLL_HARD_NEGATIVE.equals(scenario)) {
            finishWithFailure("unsupported scenario: " + scenario);
            return;
        }
        if (fixtureSourceSha256 == null
                || !fixtureSourceSha256.matches("sha256:[0-9a-f]{64}")) {
            finishWithFailure("fixtureSourceSha256 must be a prefixed lowercase SHA-256 digest");
            return;
        }
        if (!"8.13".equals(gradleVersion)) {
            finishWithFailure("gradleVersion must be 8.13");
            return;
        }
        if (!versionToken(adbVersion) || !versionToken(emulatorVersion) || !nameToken(avdName)) {
            finishWithFailure("adbVersion, emulatorVersion, and avdName are required tokens");
            return;
        }

        try {
            Context target = getTargetContext();
            Intent intent = new Intent(target, MainActivity.class);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            intent.putExtra(MainActivity.EXTRA_SCENARIO, scenario);
            Activity activity = startActivitySync(intent);
            waitForIdleSync();

            Capture capture = captureOnUiThread(
                    activity,
                    scenario,
                    fixtureSourceSha256,
                    gradleVersion,
                    adbVersion,
                    emulatorVersion,
                    avdName);
            Bitmap screenshot = getUiAutomation().takeScreenshot();
            if (screenshot == null) {
                throw new IOException("UiAutomation.takeScreenshot returned null");
            }

            File directory = new File(target.getExternalFilesDir(null), "capture");
            if (!directory.exists() && !directory.mkdirs()) {
                throw new IOException("could not create capture directory");
            }
            File screenshotFile = new File(directory, scenario + ".png");
            byte[] screenshotBytes = png(screenshot);
            writeExclusiveReplacement(screenshotFile, screenshotBytes);

            capture.screenshotWidth = screenshot.getWidth();
            capture.screenshotHeight = screenshot.getHeight();
            capture.screenshotSha256 = digest(screenshotBytes);
            capture.screenshotReference = "evaluation/android/captures/" + scenario + ".png";

            File manifestFile = new File(directory, scenario + ".capture.json");
            byte[] manifestBytes = capture.toCanonicalJson().getBytes(StandardCharsets.UTF_8);
            writeExclusiveReplacement(manifestFile, manifestBytes);

            Bundle result = new Bundle();
            result.putString("scenario", scenario);
            result.putString("manifest", manifestFile.getAbsolutePath());
            result.putString("screenshot", screenshotFile.getAbsolutePath());
            result.putString("manifestSha256", digest(manifestBytes));
            result.putString("screenshotSha256", capture.screenshotSha256);
            finish(Activity.RESULT_OK, result);
        } catch (Exception error) {
            finishWithFailure(error.getClass().getSimpleName() + ": " + error.getMessage());
        }
    }

    private Capture captureOnUiThread(
            Activity activity,
            String scenario,
            String fixtureSourceSha256,
            String gradleVersion,
            String adbVersion,
            String emulatorVersion,
            String avdName) throws Exception {
        final Capture[] result = new Capture[1];
        final Throwable[] failure = new Throwable[1];
        runOnMainSync(() -> {
            try {
                result[0] = Capture.from(
                        activity,
                        scenario,
                        fixtureSourceSha256,
                        gradleVersion,
                        adbVersion,
                        emulatorVersion,
                        avdName);
            } catch (Throwable error) {
                failure[0] = error;
            }
        });
        if (failure[0] != null) {
            throw new Exception("UI-thread capture failed", failure[0]);
        }
        return result[0];
    }

    private static byte[] png(Bitmap bitmap) throws IOException {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, output)) {
            throw new IOException("could not encode screenshot as PNG");
        }
        return output.toByteArray();
    }

    private static void writeExclusiveReplacement(File file, byte[] bytes) throws IOException {
        File temporary = new File(file.getParentFile(), file.getName() + ".tmp");
        if (temporary.exists() && !temporary.delete()) {
            throw new IOException("could not remove stale temporary file");
        }
        try (FileOutputStream output = new FileOutputStream(temporary, false)) {
            output.write(bytes);
            output.getFD().sync();
        }
        if (file.exists() && !file.delete()) {
            throw new IOException("could not replace stale capture file");
        }
        if (!temporary.renameTo(file)) {
            throw new IOException("could not publish capture file");
        }
    }

    private static String digest(byte[] bytes) {
        try {
            MessageDigest hasher = MessageDigest.getInstance("SHA-256");
            byte[] digest = hasher.digest(bytes);
            StringBuilder value = new StringBuilder("sha256:");
            for (byte item : digest) {
                value.append(String.format(Locale.ROOT, "%02x", item & 0xff));
            }
            return value.toString();
        } catch (NoSuchAlgorithmException impossible) {
            throw new IllegalStateException("Android runtime lacks SHA-256", impossible);
        }
    }

    private void finishWithFailure(String message) {
        Bundle result = new Bundle();
        result.putString("error", message == null ? "unknown capture failure" : message);
        finish(Activity.RESULT_CANCELED, result);
    }

    private static boolean versionToken(String value) {
        return value != null && value.matches("[0-9]+(?:\\.[0-9]+){1,3}");
    }

    private static boolean nameToken(String value) {
        return value != null && value.matches("[A-Za-z0-9._-]{1,64}");
    }

    private static final class Capture {
        String scenario;
        String fixtureSourceSha256;
        String gradleVersion;
        String adbVersion;
        String emulatorVersion;
        String avdName;
        int displayWidth;
        int displayHeight;
        int densityDpi;
        int rotationDegrees;
        float fontScale;
        String localeTag;
        String layoutDirection;
        String nightMode;
        InsetsFact systemBarInsets;
        List<NodeFact> nodes;
        int unidentifiedNodeCount;
        int screenshotWidth;
        int screenshotHeight;
        String screenshotReference;
        String screenshotSha256;

        static Capture from(
                Activity activity,
                String scenario,
                String fixtureSourceSha256,
                String gradleVersion,
                String adbVersion,
                String emulatorVersion,
                String avdName) {
            Capture capture = new Capture();
            capture.scenario = scenario;
            capture.fixtureSourceSha256 = fixtureSourceSha256;
            capture.gradleVersion = gradleVersion;
            capture.adbVersion = adbVersion;
            capture.emulatorVersion = emulatorVersion;
            capture.avdName = avdName;
            Display display = activity.getDisplay();
            DisplayMetrics metrics = new DisplayMetrics();
            display.getRealMetrics(metrics);
            capture.displayWidth = metrics.widthPixels;
            capture.displayHeight = metrics.heightPixels;
            capture.densityDpi = metrics.densityDpi;
            capture.rotationDegrees = rotationDegrees(display.getRotation());

            Configuration configuration = activity.getResources().getConfiguration();
            capture.fontScale = configuration.fontScale;
            capture.localeTag = configuration.getLocales().get(0).toLanguageTag();
            capture.layoutDirection = configuration.getLayoutDirection() == View.LAYOUT_DIRECTION_RTL
                    ? "rtl" : "ltr";
            int night = configuration.uiMode & Configuration.UI_MODE_NIGHT_MASK;
            capture.nightMode = night == Configuration.UI_MODE_NIGHT_YES ? "dark" : "light";

            View root = activity.findViewById(android.R.id.content).getRootView();
            WindowInsets insets = root.getRootWindowInsets();
            android.graphics.Insets systemBars = insets == null
                    ? android.graphics.Insets.NONE
                    : insets.getInsets(WindowInsets.Type.systemBars());
            capture.systemBarInsets = new InsetsFact(
                    systemBars.left, systemBars.top, systemBars.right, systemBars.bottom);

            List<NodeFact> all = new ArrayList<>();
            int[] unidentified = new int[] {0};
            collect(activity, root, null, 0, all, unidentified);
            all.sort(Comparator.comparing(node -> node.resourceId));
            capture.nodes = all;
            capture.unidentifiedNodeCount = unidentified[0];
            return capture;
        }

        String toCanonicalJson() {
            StringBuilder json = new StringBuilder();
            json.append('{');
            field(json, "captureVersion", "0.1.0");
            comma(json); field(json, "captureId", "android-atlas-" + scenario);
            comma(json); field(json, "scenario", scenario);
            comma(json); json.append("\"application\":{");
            field(json, "packageName", "org.sightlint.fixtures.atlas");
            comma(json); field(json, "versionName", "0.1.0");
            comma(json); numberField(json, "versionCode", 1);
            json.append('}');
            comma(json); json.append("\"runner\":{");
            field(json, "name", RUNNER_NAME);
            comma(json); field(json, "version", RUNNER_VERSION);
            comma(json); field(json, "captureApi", "instrumentation-view-accessibility");
            json.append('}');
            comma(json); json.append("\"build\":{");
            field(json, "fixtureSourceSha256", fixtureSourceSha256);
            comma(json); field(json, "gradleVersion", gradleVersion);
            comma(json); field(json, "androidGradlePluginVersion", "8.10.1");
            comma(json); numberField(json, "javaLanguageVersion", 17);
            comma(json); numberField(json, "compileSdk", 35);
            json.append('}');
            comma(json); json.append("\"device\":{");
            numberField(json, "apiLevel", Build.VERSION.SDK_INT);
            comma(json); field(json, "buildFingerprint", Build.FINGERPRINT);
            comma(json); field(json, "manufacturer", Build.MANUFACTURER);
            comma(json); field(json, "model", Build.MODEL);
            comma(json); field(json, "device", Build.DEVICE);
            comma(json); json.append("\"display\":{");
            numberField(json, "widthPixels", displayWidth);
            comma(json); numberField(json, "heightPixels", displayHeight);
            comma(json); numberField(json, "densityDpi", densityDpi);
            comma(json); numberField(json, "rotationDegrees", rotationDegrees);
            json.append('}');
            comma(json); json.append("\"configuration\":{");
            decimalField(json, "fontScale", fontScale);
            comma(json); field(json, "locale", localeTag);
            comma(json); field(json, "layoutDirection", layoutDirection);
            comma(json); field(json, "nightMode", nightMode);
            json.append('}');
            comma(json); json.append("\"systemBarInsetsDevicePixels\":");
            systemBarInsets.appendJson(json);
            json.append('}');
            comma(json); json.append("\"capture\":{");
            json.append("\"order\":[\"waitForIdle\",\"viewAndAccessibilityHierarchy\",\"screenshot\"]");
            comma(json); json.append("\"atomic\":false");
            comma(json); json.append("\"animationsDisabled\":true");
            comma(json); field(json, "adbVersion", adbVersion);
            comma(json); field(json, "emulatorVersion", emulatorVersion);
            comma(json); field(json, "avdName", avdName);
            comma(json); field(
                    json,
                    "instrumentationCommand",
                    "adb shell am instrument -w -e scenario <scenario> org.sightlint.fixtures.atlas.test/org.sightlint.fixtures.atlas.CaptureInstrumentation");
            comma(json); json.append("\"limitations\":[");
            string(json, "classic Android Views only"); comma(json);
            string(json, "hierarchy and screenshot are sequential, not atomic"); comma(json);
            string(json, "touch delegates and rendered node identity are not captured");
            json.append("]}");
            comma(json); json.append("\"hierarchy\":{");
            field(json, "rootResourceId", "org.sightlint.fixtures.atlas:id/atlas_root");
            comma(json); numberField(json, "unidentifiedNodeCount", unidentifiedNodeCount);
            comma(json); json.append("\"nodes\":[");
            for (int index = 0; index < nodes.size(); index++) {
                if (index > 0) comma(json);
                nodes.get(index).appendJson(json);
            }
            json.append("]}");
            comma(json); json.append("\"screenshot\":{");
            field(json, "reference", screenshotReference);
            comma(json); field(json, "sha256", screenshotSha256);
            comma(json); numberField(json, "widthPixels", screenshotWidth);
            comma(json); numberField(json, "heightPixels", screenshotHeight);
            comma(json); numberField(json, "captureSequence", 3);
            json.append("}}");
            json.append('\n');
            return json.toString();
        }

        private static int rotationDegrees(int rotation) {
            if (rotation == android.view.Surface.ROTATION_90) return 90;
            if (rotation == android.view.Surface.ROTATION_180) return 180;
            if (rotation == android.view.Surface.ROTATION_270) return 270;
            return 0;
        }
    }

    private static void collect(
            Activity activity,
            View view,
            String admittedParent,
            int depth,
            List<NodeFact> nodes,
            int[] unidentified) {
        String resourceId = resourceName(activity, view);
        String nextParent = admittedParent;
        if (resourceId == null) {
            unidentified[0]++;
        } else {
            NodeFact fact = NodeFact.from(view, resourceId, admittedParent, depth);
            nodes.add(fact);
            nextParent = resourceId;
        }
        if (view instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) view;
            for (int index = 0; index < group.getChildCount(); index++) {
                collect(activity, group.getChildAt(index), nextParent, depth + 1, nodes, unidentified);
            }
        }
    }

    private static String resourceName(Activity activity, View view) {
        if (view.getId() == View.NO_ID) return null;
        try {
            return activity.getResources().getResourceName(view.getId());
        } catch (android.content.res.Resources.NotFoundException ignored) {
            return null;
        }
    }

    private static final class NodeFact {
        String resourceId;
        String parentResourceId;
        int depth;
        String className;
        RectFact layoutBounds;
        boolean identityTransform;
        boolean globalVisible;
        RectFact globalVisibleBounds;
        boolean shown;
        boolean enabled;
        boolean clickable;
        boolean focusable;
        boolean focused;
        boolean selected;
        boolean checkable;
        boolean checked;
        boolean scrollable;
        boolean longClickable;
        StringFact text;
        StringFact contentDescription;
        AccessibilityFact accessibility;

        static NodeFact from(View view, String resourceId, String parent, int depth) {
            NodeFact fact = new NodeFact();
            fact.resourceId = resourceId;
            fact.parentResourceId = parent;
            fact.depth = depth;
            fact.className = view.getClass().getName();
            int[] location = new int[2];
            view.getLocationOnScreen(location);
            fact.layoutBounds = new RectFact(location[0], location[1], view.getWidth(), view.getHeight());
            fact.identityTransform = view.getMatrix().isIdentity();
            Rect visible = new Rect();
            fact.globalVisible = view.getGlobalVisibleRect(visible);
            fact.globalVisibleBounds = fact.globalVisible
                    ? new RectFact(visible.left, visible.top, visible.width(), visible.height()) : null;
            fact.shown = view.isShown();
            fact.enabled = view.isEnabled();
            fact.clickable = view.isClickable();
            fact.focusable = view.isFocusable();
            fact.focused = view.isFocused();
            fact.selected = view.isSelected();
            fact.longClickable = view.isLongClickable();
            fact.scrollable = view.canScrollHorizontally(-1) || view.canScrollHorizontally(1)
                    || view.canScrollVertically(-1) || view.canScrollVertically(1);
            AccessibilityNodeInfo info = view.createAccessibilityNodeInfo();
            fact.checkable = info != null && info.isCheckable();
            fact.checked = info != null && info.isChecked();
            fact.text = view instanceof TextView
                    ? StringFact.from(((TextView) view).getText()) : null;
            fact.contentDescription = StringFact.from(view.getContentDescription());
            fact.accessibility = AccessibilityFact.from(info);
            if (info != null) info.recycle();
            return fact;
        }

        void appendJson(StringBuilder json) {
            json.append('{');
            field(json, "resourceId", resourceId);
            comma(json); nullableField(json, "parentResourceId", parentResourceId);
            comma(json); numberField(json, "depth", depth);
            comma(json); field(json, "className", className);
            comma(json); json.append("\"layoutBoundsDevicePixels\":"); layoutBounds.appendJson(json);
            comma(json); booleanField(json, "identityTransform", identityTransform);
            comma(json); json.append("\"globalVisible\":{");
            booleanField(json, "value", globalVisible);
            comma(json); json.append("\"boundsDevicePixels\":");
            if (globalVisibleBounds == null) json.append("null"); else globalVisibleBounds.appendJson(json);
            json.append('}');
            comma(json); json.append("\"viewState\":{");
            booleanField(json, "shown", shown);
            comma(json); booleanField(json, "enabled", enabled);
            comma(json); booleanField(json, "clickable", clickable);
            comma(json); booleanField(json, "focusable", focusable);
            comma(json); booleanField(json, "focused", focused);
            comma(json); booleanField(json, "selected", selected);
            comma(json); booleanField(json, "checkable", checkable);
            comma(json); booleanField(json, "checked", checked);
            comma(json); booleanField(json, "scrollable", scrollable);
            comma(json); booleanField(json, "longClickable", longClickable);
            json.append('}');
            comma(json); json.append("\"text\":"); appendNullable(json, text);
            comma(json); json.append("\"contentDescription\":"); appendNullable(json, contentDescription);
            comma(json); json.append("\"accessibility\":"); accessibility.appendJson(json);
            json.append('}');
        }
    }

    private static final class AccessibilityFact {
        String className;
        String packageName;
        String viewIdResourceName;
        RectFact bounds;
        RawBoundsFact rawBounds;
        String geometryStatus;
        List<Integer> actions;
        boolean enabled;
        boolean clickable;
        boolean focusable;
        boolean focused;
        boolean selected;
        boolean checkable;
        boolean checked;
        boolean scrollable;
        boolean longClickable;
        boolean visibleToUser;

        static AccessibilityFact from(AccessibilityNodeInfo info) {
            AccessibilityFact fact = new AccessibilityFact();
            if (info == null) {
                fact.actions = new ArrayList<>();
                fact.geometryStatus = "unavailable";
                return fact;
            }
            fact.className = stringValue(info.getClassName());
            fact.packageName = stringValue(info.getPackageName());
            fact.viewIdResourceName = info.getViewIdResourceName();
            Rect bounds = new Rect();
            info.getBoundsInScreen(bounds);
            fact.rawBounds = new RawBoundsFact(
                    bounds.left, bounds.top, bounds.right, bounds.bottom);
            if (bounds.right >= bounds.left && bounds.bottom >= bounds.top) {
                fact.geometryStatus = "exact";
                fact.bounds = new RectFact(
                        bounds.left, bounds.top, bounds.right - bounds.left, bounds.bottom - bounds.top);
            } else {
                fact.geometryStatus = "invalidPlatformBounds";
                fact.bounds = null;
            }
            fact.actions = new ArrayList<>();
            for (AccessibilityNodeInfo.AccessibilityAction action : info.getActionList()) {
                fact.actions.add(action.getId());
            }
            fact.actions.sort(Integer::compareTo);
            fact.enabled = info.isEnabled();
            fact.clickable = info.isClickable();
            fact.focusable = info.isFocusable();
            fact.focused = info.isFocused();
            fact.selected = info.isSelected();
            fact.checkable = info.isCheckable();
            fact.checked = info.isChecked();
            fact.scrollable = info.isScrollable();
            fact.longClickable = info.isLongClickable();
            fact.visibleToUser = info.isVisibleToUser();
            return fact;
        }

        void appendJson(StringBuilder json) {
            json.append('{');
            nullableField(json, "className", className);
            comma(json); nullableField(json, "packageName", packageName);
            comma(json); nullableField(json, "viewIdResourceName", viewIdResourceName);
            comma(json); field(json, "geometryStatus", geometryStatus);
            comma(json); json.append("\"rawBoundsDevicePixels\":");
            if (rawBounds == null) json.append("null"); else rawBounds.appendJson(json);
            comma(json); json.append("\"boundsDevicePixels\":");
            if (bounds == null) json.append("null"); else bounds.appendJson(json);
            comma(json); json.append("\"actionIds\":[");
            for (int index = 0; index < actions.size(); index++) {
                if (index > 0) comma(json);
                json.append(actions.get(index));
            }
            json.append(']');
            comma(json); booleanField(json, "enabled", enabled);
            comma(json); booleanField(json, "clickable", clickable);
            comma(json); booleanField(json, "focusable", focusable);
            comma(json); booleanField(json, "focused", focused);
            comma(json); booleanField(json, "selected", selected);
            comma(json); booleanField(json, "checkable", checkable);
            comma(json); booleanField(json, "checked", checked);
            comma(json); booleanField(json, "scrollable", scrollable);
            comma(json); booleanField(json, "longClickable", longClickable);
            comma(json); booleanField(json, "visibleToUser", visibleToUser);
            json.append('}');
        }
    }

    private static final class RectFact {
        final int x;
        final int y;
        final int width;
        final int height;

        RectFact(int x, int y, int width, int height) {
            this.x = x;
            this.y = y;
            this.width = width;
            this.height = height;
        }

        void appendJson(StringBuilder json) {
            json.append('{');
            numberField(json, "x", x);
            comma(json); numberField(json, "y", y);
            comma(json); numberField(json, "width", width);
            comma(json); numberField(json, "height", height);
            json.append('}');
        }
    }

    private static final class RawBoundsFact {
        final int left;
        final int top;
        final int right;
        final int bottom;

        RawBoundsFact(int left, int top, int right, int bottom) {
            this.left = left;
            this.top = top;
            this.right = right;
            this.bottom = bottom;
        }

        void appendJson(StringBuilder json) {
            json.append('{');
            numberField(json, "left", left);
            comma(json); numberField(json, "top", top);
            comma(json); numberField(json, "right", right);
            comma(json); numberField(json, "bottom", bottom);
            json.append('}');
        }
    }

    private static final class InsetsFact {
        final int left;
        final int top;
        final int right;
        final int bottom;

        InsetsFact(int left, int top, int right, int bottom) {
            this.left = left;
            this.top = top;
            this.right = right;
            this.bottom = bottom;
        }

        void appendJson(StringBuilder json) {
            json.append('{');
            numberField(json, "left", left);
            comma(json); numberField(json, "top", top);
            comma(json); numberField(json, "right", right);
            comma(json); numberField(json, "bottom", bottom);
            json.append('}');
        }
    }

    private static final class StringFact {
        final int utf8ByteLength;
        final String sha256;

        StringFact(int utf8ByteLength, String sha256) {
            this.utf8ByteLength = utf8ByteLength;
            this.sha256 = sha256;
        }

        static StringFact from(CharSequence value) {
            if (value == null || value.length() == 0) return null;
            byte[] bytes = value.toString().getBytes(StandardCharsets.UTF_8);
            return new StringFact(bytes.length, digest(bytes));
        }

        void appendJson(StringBuilder json) {
            json.append('{');
            numberField(json, "utf8ByteLength", utf8ByteLength);
            comma(json); field(json, "sha256", sha256);
            json.append('}');
        }
    }

    private static String stringValue(CharSequence value) {
        return value == null ? null : value.toString();
    }

    private static void appendNullable(StringBuilder json, StringFact value) {
        if (value == null) json.append("null"); else value.appendJson(json);
    }

    private static void field(StringBuilder json, String name, String value) {
        string(json, name); json.append(':'); string(json, value);
    }

    private static void nullableField(StringBuilder json, String name, String value) {
        string(json, name); json.append(':');
        if (value == null) json.append("null"); else string(json, value);
    }

    private static void numberField(StringBuilder json, String name, int value) {
        string(json, name); json.append(':').append(value);
    }

    private static void decimalField(StringBuilder json, String name, float value) {
        string(json, name); json.append(':').append(Float.toString(value));
    }

    private static void booleanField(StringBuilder json, String name, boolean value) {
        string(json, name); json.append(':').append(value ? "true" : "false");
    }

    private static void string(StringBuilder json, String value) {
        json.append(JSONObject.quote(value));
    }

    private static void comma(StringBuilder json) {
        json.append(',');
    }
}
