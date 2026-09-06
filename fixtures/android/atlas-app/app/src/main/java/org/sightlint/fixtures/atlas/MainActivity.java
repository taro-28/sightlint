package org.sightlint.fixtures.atlas;

import android.app.Activity;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Space;
import android.widget.Switch;
import android.widget.TextView;

public final class MainActivity extends Activity {
    public static final String EXTRA_SCENARIO = "org.sightlint.fixtures.atlas.SCENARIO";
    public static final String CLEAN = "clean";
    public static final String OFF_CANVAS = "off-canvas-control-mutant";
    public static final String SCROLL_HARD_NEGATIVE = "scroll-offscreen-hard-negative";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        configureStaticCaptureWindow();
        String scenario = getIntent().getStringExtra(EXTRA_SCENARIO);
        if (!OFF_CANVAS.equals(scenario) && !SCROLL_HARD_NEGATIVE.equals(scenario)) {
            scenario = CLEAN;
        }
        setContentView(buildScreen(scenario));
    }

    @SuppressWarnings("deprecation")
    private void configureStaticCaptureWindow() {
        getWindow().setStatusBarColor(getColor(R.color.atlas_background));
        getWindow().setNavigationBarColor(getColor(R.color.atlas_background));
        getWindow().getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                        | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                        | View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR);
    }

    private View buildScreen(String scenario) {
        ScrollView root = new ScrollView(this);
        root.setId(R.id.atlas_root);
        root.setFillViewport(true);
        root.setBackgroundColor(getColor(R.color.atlas_background));
        root.setClipToPadding(true);

        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(dp(24), dp(28), dp(24), dp(28));
        root.addView(content, new ScrollView.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        TextView title = text(R.id.screen_title, getString(R.string.screen_title), 28, true);
        content.addView(title, matchWrap());

        TextView subtitle = text(
                R.id.screen_subtitle, getString(R.string.screen_subtitle), 15, false);
        subtitle.setTextColor(getColor(R.color.atlas_muted));
        content.addView(subtitle, marginTop(8));

        LinearLayout accountCard = new LinearLayout(this);
        accountCard.setId(R.id.account_card);
        accountCard.setGravity(Gravity.CENTER_VERTICAL);
        accountCard.setOrientation(LinearLayout.HORIZONTAL);
        accountCard.setPadding(dp(18), dp(18), dp(18), dp(18));
        accountCard.setBackground(roundedSurface());
        content.addView(accountCard, marginTopHeight(24, dp(104)));

        TextView avatar = text(R.id.avatar, "AM", 18, true);
        avatar.setContentDescription(getString(R.string.avatar_description));
        avatar.setGravity(Gravity.CENTER);
        GradientDrawable avatarBackground = new GradientDrawable();
        avatarBackground.setShape(GradientDrawable.OVAL);
        avatarBackground.setColor(getColor(R.color.atlas_primary));
        avatar.setTextColor(Color.WHITE);
        avatar.setBackground(avatarBackground);
        accountCard.addView(avatar, new LinearLayout.LayoutParams(dp(56), dp(56)));

        LinearLayout identity = new LinearLayout(this);
        identity.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams identityParams = new LinearLayout.LayoutParams(
                0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
        identityParams.setMarginStart(dp(16));
        accountCard.addView(identity, identityParams);

        identity.addView(
                text(R.id.account_name, getString(R.string.account_name), 18, true), matchWrap());
        TextView plan = text(R.id.account_plan, getString(R.string.account_plan), 14, false);
        plan.setTextColor(getColor(R.color.atlas_muted));
        identity.addView(plan, marginTop(4));

        LinearLayout settings = new LinearLayout(this);
        settings.setId(R.id.settings_section);
        settings.setOrientation(LinearLayout.VERTICAL);
        settings.setPadding(dp(18), dp(6), dp(18), dp(6));
        settings.setBackground(roundedSurface());
        content.addView(settings, marginTop(18));

        settings.addView(notificationRow(), rowParams());
        View divider = new View(this);
        divider.setBackgroundColor(getColor(R.color.atlas_border));
        settings.addView(divider, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(1)));
        settings.addView(privacyRow(), rowParams());

        FrameLayout actionRegion = new FrameLayout(this);
        actionRegion.setId(R.id.action_region);
        actionRegion.setClipChildren(true);
        content.addView(actionRegion, marginTopHeight(24, dp(56)));

        Button save = new Button(this);
        save.setId(R.id.save_button);
        save.setText(R.string.save_changes);
        save.setTextColor(Color.WHITE);
        save.setTextSize(15);
        save.setAllCaps(false);
        save.setBackground(roundedPrimary());
        FrameLayout.LayoutParams saveParams = new FrameLayout.LayoutParams(dp(220), dp(56));
        saveParams.gravity = Gravity.START | Gravity.TOP;
        saveParams.leftMargin = OFF_CANVAS.equals(scenario) ? dp(280) : 0;
        actionRegion.addView(save, saveParams);

        if (SCROLL_HARD_NEGATIVE.equals(scenario)) {
            Space spacer = new Space(this);
            spacer.setId(R.id.archive_spacer);
            content.addView(spacer, new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(780)));
            LinearLayout archived = new LinearLayout(this);
            archived.setId(R.id.archived_section);
            archived.setOrientation(LinearLayout.VERTICAL);
            archived.setPadding(dp(18), dp(18), dp(18), dp(18));
            archived.setBackground(roundedSurface());
            archived.addView(text(
                    R.id.archived_label, getString(R.string.archived_label), 16, true), matchWrap());
            content.addView(archived, new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(88)));
        }

        return root;
    }

    @SuppressWarnings("deprecation")
    private View notificationRow() {
        LinearLayout row = new LinearLayout(this);
        row.setId(R.id.notification_row);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setOrientation(LinearLayout.HORIZONTAL);
        TextView label = text(
                R.id.notification_label, getString(R.string.notifications), 16, false);
        row.addView(label, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        Switch toggle = new Switch(this);
        toggle.setId(R.id.notification_switch);
        toggle.setChecked(true);
        toggle.setContentDescription(getString(R.string.notifications));
        row.addView(toggle, new LinearLayout.LayoutParams(dp(64), dp(48)));
        return row;
    }

    private View privacyRow() {
        LinearLayout row = new LinearLayout(this);
        row.setId(R.id.privacy_row);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setClickable(true);
        row.setFocusable(true);
        TextView label = text(R.id.privacy_label, getString(R.string.privacy), 16, false);
        row.addView(label, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView value = text(R.id.privacy_value, getString(R.string.privacy_value), 14, false);
        value.setTextColor(getColor(R.color.atlas_muted));
        row.addView(value, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        return row;
    }

    private TextView text(int id, String value, int sizeSp, boolean strong) {
        TextView view = new TextView(this);
        view.setId(id);
        view.setText(value);
        view.setTextColor(getColor(R.color.atlas_text));
        view.setTextSize(sizeSp);
        if (strong) {
            view.setTypeface(view.getTypeface(), android.graphics.Typeface.BOLD);
        }
        return view;
    }

    private GradientDrawable roundedSurface() {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(getColor(R.color.atlas_surface));
        drawable.setCornerRadius(dp(18));
        drawable.setStroke(dp(1), getColor(R.color.atlas_border));
        return drawable;
    }

    private GradientDrawable roundedPrimary() {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(getColor(R.color.atlas_primary));
        drawable.setCornerRadius(dp(16));
        return drawable;
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    private LinearLayout.LayoutParams marginTop(int dp) {
        LinearLayout.LayoutParams params = matchWrap();
        params.topMargin = dp(dp);
        return params;
    }

    private LinearLayout.LayoutParams marginTopHeight(int topDp, int heightPx) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, heightPx);
        params.topMargin = dp(topDp);
        return params;
    }

    private LinearLayout.LayoutParams rowParams() {
        return new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(72));
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
