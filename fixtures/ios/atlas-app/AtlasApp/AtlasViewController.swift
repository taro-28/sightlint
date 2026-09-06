import UIKit

final class AtlasViewController: UIViewController {
    private let scenario: CaptureScenario
    private let content = UIView()
    private let scrollView = UIScrollView()
    private let readyLabel = UILabel()

    init(scenario: CaptureScenario) {
        self.scenario = scenario
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is unavailable")
    }

    override var preferredStatusBarStyle: UIStatusBarStyle { .darkContent }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = UIColor(red: 0.957, green: 0.969, blue: 0.984, alpha: 1)
        view.accessibilityIdentifier = "screen_root"
        buildInterface()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        view.layoutIfNeeded()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
            guard let self, let window = self.view.window else { return }
            do {
                self.readyLabel.text = "Ready"
                self.readyLabel.accessibilityLabel = "Capture ready"
                try SourceCapture.write(root: self.view, window: window)
            } catch {
                self.readyLabel.text = "Capture unavailable"
                self.readyLabel.accessibilityLabel = "Capture unavailable"
            }
        }
    }

    private func identified<T: UIView>(_ view: T, _ identifier: String) -> T {
        view.accessibilityIdentifier = identifier
        view.translatesAutoresizingMaskIntoConstraints = false
        return view
    }

    private func label(_ text: String, identifier: String, size: CGFloat, weight: UIFont.Weight) -> UILabel {
        let label = identified(UILabel(), identifier)
        label.text = text
        label.font = UIFont.systemFont(ofSize: size, weight: weight)
        label.textColor = UIColor(red: 0.075, green: 0.102, blue: 0.149, alpha: 1)
        label.numberOfLines = 1
        label.isAccessibilityElement = true
        return label
    }

    private func card(identifier: String) -> UIView {
        let card = identified(UIView(), identifier)
        card.backgroundColor = .white
        card.layer.cornerRadius = 16
        card.layer.borderColor = UIColor(red: 0.867, green: 0.89, blue: 0.925, alpha: 1).cgColor
        card.layer.borderWidth = 1
        card.isAccessibilityElement = false
        return card
    }

    private func buildInterface() {
        scrollView.accessibilityIdentifier = "settings_scroll"
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.alwaysBounceVertical = false
        scrollView.alwaysBounceHorizontal = false
        scrollView.contentInsetAdjustmentBehavior = .never
        scrollView.isDirectionalLockEnabled = true
        scrollView.backgroundColor = .clear
        view.addSubview(scrollView)

        content.accessibilityIdentifier = "settings_content"
        content.translatesAutoresizingMaskIntoConstraints = false
        scrollView.addSubview(content)

        let title = label("Account settings", identifier: "account_title", size: 28, weight: .bold)
        let subtitle = label("Manage your profile and workspace preferences", identifier: "account_subtitle", size: 14, weight: .regular)
        subtitle.textColor = UIColor(red: 0.36, green: 0.408, blue: 0.486, alpha: 1)

        let profile = card(identifier: "profile_card")
        let avatar = identified(UIView(), "profile_avatar")
        avatar.backgroundColor = UIColor(red: 0.255, green: 0.353, blue: 0.933, alpha: 1)
        avatar.layer.cornerRadius = 24
        avatar.isAccessibilityElement = false
        let initials = label("AM", identifier: "profile_initials", size: 16, weight: .bold)
        initials.textColor = .white
        initials.textAlignment = .center
        avatar.addSubview(initials)
        let name = label("Alex Morgan", identifier: "profile_name", size: 17, weight: .semibold)
        let email = label("alex@example.test", identifier: "profile_email", size: 13, weight: .regular)
        email.textColor = UIColor(red: 0.36, green: 0.408, blue: 0.486, alpha: 1)
        let plan = label("Workspace plan", identifier: "profile_plan", size: 12, weight: .semibold)
        plan.textColor = UIColor(red: 0.255, green: 0.353, blue: 0.933, alpha: 1)

        profile.addSubview(avatar)
        profile.addSubview(name)
        profile.addSubview(email)
        profile.addSubview(plan)

        let preferencesTitle = label("Preferences", identifier: "preferences_title", size: 19, weight: .bold)
        let preferences = card(identifier: "preferences_card")
        let notifications = label("Product notifications", identifier: "notifications_label", size: 16, weight: .medium)
        let notificationsDetail = label("News and weekly summaries", identifier: "notifications_detail", size: 12, weight: .regular)
        notificationsDetail.textColor = UIColor(red: 0.36, green: 0.408, blue: 0.486, alpha: 1)
        let notificationsSwitch = identified(UISwitch(), "notifications_switch")
        notificationsSwitch.isOn = true
        notificationsSwitch.accessibilityLabel = "Product notifications"
        let divider = identified(UIView(), "preferences_divider")
        divider.backgroundColor = UIColor(red: 0.9, green: 0.914, blue: 0.941, alpha: 1)
        let visibility = label("Profile visibility", identifier: "visibility_label", size: 16, weight: .medium)
        let visibilityDetail = label("Visible to workspace members", identifier: "visibility_detail", size: 12, weight: .regular)
        visibilityDetail.textColor = UIColor(red: 0.36, green: 0.408, blue: 0.486, alpha: 1)
        let visibilityValue = label("Workspace", identifier: "visibility_value", size: 14, weight: .semibold)
        visibilityValue.textColor = UIColor(red: 0.255, green: 0.353, blue: 0.933, alpha: 1)

        for item in [notifications, notificationsDetail, notificationsSwitch, divider, visibility, visibilityDetail, visibilityValue] {
            preferences.addSubview(item)
        }

        let save = identified(UIButton(type: .system), "save_button")
        save.setTitle("Save changes", for: .normal)
        save.titleLabel?.font = UIFont.systemFont(ofSize: 17, weight: .semibold)
        save.setTitleColor(.white, for: .normal)
        save.backgroundColor = UIColor(red: 0.255, green: 0.353, blue: 0.933, alpha: 1)
        save.layer.cornerRadius = 12

        readyLabel.accessibilityIdentifier = "capture_ready"
        readyLabel.translatesAutoresizingMaskIntoConstraints = false
        readyLabel.text = "Preparing capture"
        readyLabel.font = UIFont.systemFont(ofSize: 11, weight: .regular)
        readyLabel.textColor = UIColor(red: 0.36, green: 0.408, blue: 0.486, alpha: 1)
        readyLabel.isAccessibilityElement = true

        for item in [title, subtitle, profile, preferencesTitle, preferences, save, readyLabel] {
            content.addSubview(item)
        }

        let saveLeading: CGFloat = scenario == .offCanvasControlMutant ? 300 : 24
        var constraints = [
            scrollView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            scrollView.topAnchor.constraint(equalTo: view.topAnchor),
            scrollView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            content.leadingAnchor.constraint(equalTo: scrollView.contentLayoutGuide.leadingAnchor),
            content.trailingAnchor.constraint(equalTo: scrollView.contentLayoutGuide.trailingAnchor),
            content.topAnchor.constraint(equalTo: scrollView.contentLayoutGuide.topAnchor),
            content.bottomAnchor.constraint(equalTo: scrollView.contentLayoutGuide.bottomAnchor),
            content.widthAnchor.constraint(equalTo: scrollView.frameLayoutGuide.widthAnchor),
            title.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            title.topAnchor.constraint(equalTo: content.topAnchor, constant: 82),
            subtitle.leadingAnchor.constraint(equalTo: title.leadingAnchor),
            subtitle.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 6),
            profile.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            profile.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),
            profile.topAnchor.constraint(equalTo: subtitle.bottomAnchor, constant: 22),
            profile.heightAnchor.constraint(equalToConstant: 104),
            avatar.leadingAnchor.constraint(equalTo: profile.leadingAnchor, constant: 18),
            avatar.centerYAnchor.constraint(equalTo: profile.centerYAnchor),
            avatar.widthAnchor.constraint(equalToConstant: 48),
            avatar.heightAnchor.constraint(equalToConstant: 48),
            initials.leadingAnchor.constraint(equalTo: avatar.leadingAnchor),
            initials.trailingAnchor.constraint(equalTo: avatar.trailingAnchor),
            initials.centerYAnchor.constraint(equalTo: avatar.centerYAnchor),
            name.leadingAnchor.constraint(equalTo: avatar.trailingAnchor, constant: 14),
            name.topAnchor.constraint(equalTo: profile.topAnchor, constant: 20),
            email.leadingAnchor.constraint(equalTo: name.leadingAnchor),
            email.topAnchor.constraint(equalTo: name.bottomAnchor, constant: 4),
            plan.leadingAnchor.constraint(equalTo: name.leadingAnchor),
            plan.topAnchor.constraint(equalTo: email.bottomAnchor, constant: 7),
            preferencesTitle.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            preferencesTitle.topAnchor.constraint(equalTo: profile.bottomAnchor, constant: 28),
            preferences.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            preferences.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),
            preferences.topAnchor.constraint(equalTo: preferencesTitle.bottomAnchor, constant: 12),
            preferences.heightAnchor.constraint(equalToConstant: 168),
            notifications.leadingAnchor.constraint(equalTo: preferences.leadingAnchor, constant: 18),
            notifications.topAnchor.constraint(equalTo: preferences.topAnchor, constant: 22),
            notificationsDetail.leadingAnchor.constraint(equalTo: notifications.leadingAnchor),
            notificationsDetail.topAnchor.constraint(equalTo: notifications.bottomAnchor, constant: 4),
            notificationsSwitch.trailingAnchor.constraint(equalTo: preferences.trailingAnchor, constant: -18),
            notificationsSwitch.centerYAnchor.constraint(equalTo: notifications.centerYAnchor, constant: 8),
            divider.leadingAnchor.constraint(equalTo: preferences.leadingAnchor, constant: 18),
            divider.trailingAnchor.constraint(equalTo: preferences.trailingAnchor, constant: -18),
            divider.topAnchor.constraint(equalTo: preferences.topAnchor, constant: 84),
            divider.heightAnchor.constraint(equalToConstant: 1),
            visibility.leadingAnchor.constraint(equalTo: preferences.leadingAnchor, constant: 18),
            visibility.topAnchor.constraint(equalTo: divider.bottomAnchor, constant: 20),
            visibilityDetail.leadingAnchor.constraint(equalTo: visibility.leadingAnchor),
            visibilityDetail.topAnchor.constraint(equalTo: visibility.bottomAnchor, constant: 4),
            visibilityValue.trailingAnchor.constraint(equalTo: preferences.trailingAnchor, constant: -18),
            visibilityValue.centerYAnchor.constraint(equalTo: visibility.centerYAnchor, constant: 8),
            save.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: saveLeading),
            save.topAnchor.constraint(equalTo: preferences.bottomAnchor, constant: 26),
            save.widthAnchor.constraint(equalToConstant: 354),
            save.heightAnchor.constraint(equalToConstant: 52),
            readyLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            readyLabel.topAnchor.constraint(equalTo: save.bottomAnchor, constant: 16)
        ]

        if scenario == .scrollOffscreenHardNegative {
            let archivedCard = card(identifier: "archived_card")
            let archivedTitle = label("Archived preferences", identifier: "archived_title", size: 17, weight: .semibold)
            let archivedDetail = label("Older workspace controls", identifier: "archived_detail", size: 13, weight: .regular)
            archivedDetail.textColor = UIColor(red: 0.36, green: 0.408, blue: 0.486, alpha: 1)
            archivedCard.addSubview(archivedTitle)
            archivedCard.addSubview(archivedDetail)
            content.addSubview(archivedCard)
            constraints += [
                archivedCard.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
                archivedCard.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),
                archivedCard.topAnchor.constraint(equalTo: content.topAnchor, constant: 930),
                archivedCard.heightAnchor.constraint(equalToConstant: 92),
                archivedTitle.leadingAnchor.constraint(equalTo: archivedCard.leadingAnchor, constant: 18),
                archivedTitle.topAnchor.constraint(equalTo: archivedCard.topAnchor, constant: 18),
                archivedDetail.leadingAnchor.constraint(equalTo: archivedTitle.leadingAnchor),
                archivedDetail.topAnchor.constraint(equalTo: archivedTitle.bottomAnchor, constant: 6),
                content.bottomAnchor.constraint(equalTo: archivedCard.bottomAnchor, constant: 36)
            ]
        } else {
            constraints.append(content.bottomAnchor.constraint(equalTo: readyLabel.bottomAnchor, constant: 36))
        }
        NSLayoutConstraint.activate(constraints)
    }
}
