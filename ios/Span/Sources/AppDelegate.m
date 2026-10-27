#import "AppDelegate.h"

@implementation AppDelegate

- (BOOL)application:(UIApplication *)application didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    (void)application;
    (void)launchOptions;
    CGRect bounds = [UIScreen mainScreen].bounds;
    self.window = [[UIWindow alloc] initWithFrame:bounds];
    UIViewController *controller = [[UIViewController alloc] init];
    controller.view.backgroundColor = [UIColor colorWithWhite:0.08 alpha:1.0];

    UILabel *label = [[UILabel alloc] initWithFrame:CGRectInset(bounds, 24.0, 24.0)];
    label.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    label.textColor = [UIColor whiteColor];
    label.textAlignment = NSTextAlignmentCenter;
    label.numberOfLines = 0;
    label.font = [UIFont systemFontOfSize:22.0];
#if SPAN_LEGACY
    label.text = @"Span\n\nЭта сборка для iOS 9.\nНа компьютере запустите span host и сопрягите устройство по QR.";
#else
    label.text = @"Span\n\nЭта сборка для iOS 15 и новее.\nНа компьютере запустите span host и сопрягите устройство по QR.";
#endif
    [controller.view addSubview:label];
    self.window.rootViewController = controller;
    [self.window makeKeyAndVisible];
    return YES;
}

@end
