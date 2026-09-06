import CryptoKit
import UIKit

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        configurationForConnecting connectingSceneSession: UISceneSession,
        options: UIScene.ConnectionOptions
    ) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(
            name: "Default Configuration",
            sessionRole: connectingSceneSession.role
        )
        configuration.delegateClass = SceneDelegate.self
        return configuration
    }
}

enum CaptureScenario: String {
    case clean
    case offCanvasControlMutant = "off-canvas-control-mutant"
    case scrollOffscreenHardNegative = "scroll-offscreen-hard-negative"

    static var current: CaptureScenario {
        let value = ProcessInfo.processInfo.environment["SIGHTLINT_SCENARIO"] ?? clean.rawValue
        return CaptureScenario(rawValue: value) ?? .clean
    }
}

struct StringFact: Encodable {
    let sha256: String
    let utf8ByteLength: Int

    init?(_ value: String?) {
        guard let value, !value.isEmpty else {
            return nil
        }
        let data = Data(value.utf8)
        sha256 = "sha256:" + SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        utf8ByteLength = data.count
    }
}

struct PointRect: Encodable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double

    init(_ rect: CGRect) {
        x = PointRect.round(rect.origin.x)
        y = PointRect.round(rect.origin.y)
        width = PointRect.round(rect.size.width)
        height = PointRect.round(rect.size.height)
    }

    private static func round(_ value: CGFloat) -> Double {
        let result = (Double(value) * 1000).rounded() / 1000
        return result == -0 ? 0 : result
    }
}

struct SourceState: Encodable {
    let alpha: Double
    let enabled: Bool?
    let hidden: Bool
    let selected: Bool?
    let userInteractionEnabled: Bool
    let windowAttached: Bool
}

struct SourceNode: Encodable {
    let className: String
    let depth: Int
    let identifier: String
    let identityTransform: Bool
    let label: StringFact?
    let layoutBoundsPoints: PointRect
    let parentIdentifier: String?
    let safeAreaIntersectionPoints: PointRect?
    let state: SourceState
    let value: StringFact?
    let windowIntersectionPoints: PointRect?
}

struct SourceHierarchy: Encodable {
    let nodes: [SourceNode]
    let rootIdentifier: String
    let unidentifiedNodeCount: Int
}

@MainActor
enum SourceCapture {
    static func write(root: UIView, window: UIWindow) throws {
        var nodes: [SourceNode] = []
        var unidentified = 0
        let screenRect = window.bounds
        let safeRect = screenRect.inset(by: window.safeAreaInsets)

        func intersection(_ rect: CGRect, with boundary: CGRect) -> PointRect? {
            let clipped = rect.intersection(boundary)
            guard !clipped.isNull, !clipped.isInfinite, !clipped.isEmpty else {
                return nil
            }
            return PointRect(clipped)
        }

        func visit(_ view: UIView, parentIdentifier: String?, depth: Int) {
            let identifier = view.accessibilityIdentifier ?? ""
            let nextParent: String?
            if identifier.isEmpty {
                unidentified += 1
                nextParent = parentIdentifier
            } else {
                let sourceRect = view.convert(view.bounds, to: window)
                let control = view as? UIControl
                let labelText: String?
                if let label = view as? UILabel {
                    labelText = label.text
                } else if let button = view as? UIButton {
                    labelText = button.title(for: .normal)
                } else {
                    labelText = view.accessibilityLabel
                }
                let valueText: String?
                if let textField = view as? UITextField {
                    valueText = textField.text
                } else if let toggle = view as? UISwitch {
                    valueText = toggle.isOn ? "1" : "0"
                } else {
                    valueText = view.accessibilityValue
                }
                nodes.append(
                    SourceNode(
                        className: String(describing: type(of: view)),
                        depth: depth,
                        identifier: identifier,
                        identityTransform: view.transform.isIdentity,
                        label: StringFact(labelText),
                        layoutBoundsPoints: PointRect(sourceRect),
                        parentIdentifier: parentIdentifier,
                        safeAreaIntersectionPoints: intersection(sourceRect, with: safeRect),
                        state: SourceState(
                            alpha: Double(view.alpha),
                            enabled: control?.isEnabled,
                            hidden: view.isHidden,
                            selected: control?.isSelected,
                            userInteractionEnabled: view.isUserInteractionEnabled,
                            windowAttached: view.window === window
                        ),
                        value: StringFact(valueText),
                        windowIntersectionPoints: intersection(sourceRect, with: screenRect)
                    )
                )
                nextParent = identifier
            }
            for child in view.subviews {
                visit(child, parentIdentifier: nextParent, depth: depth + 1)
            }
        }

        visit(root, parentIdentifier: nil, depth: 0)
        let capture = SourceHierarchy(
            nodes: nodes.sorted { $0.identifier < $1.identifier },
            rootIdentifier: root.accessibilityIdentifier ?? "screen_root",
            unidentifiedNodeCount: unidentified
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        let data = try encoder.encode(capture)
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        try data.write(to: documents.appendingPathComponent("sightlint-source.json"), options: .atomic)
    }
}
