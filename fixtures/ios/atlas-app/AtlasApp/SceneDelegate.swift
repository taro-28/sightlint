import UIKit

@MainActor
final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?

    func scene(
        _ scene: UIScene,
        willConnectTo session: UISceneSession,
        options connectionOptions: UIScene.ConnectionOptions
    ) {
        guard let windowScene = scene as? UIWindowScene else { return }
        UIView.setAnimationsEnabled(false)
        let window = UIWindow(windowScene: windowScene)
        window.rootViewController = AtlasViewController(scenario: CaptureScenario.current)
        window.makeKeyAndVisible()
        self.window = window
    }
}
