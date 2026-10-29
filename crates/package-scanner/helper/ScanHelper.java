import android.app.Application;
import android.app.Instrumentation;
import android.content.Context;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.content.pm.SigningInfo;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.net.http.SslCertificate;
import android.os.Build;
import android.os.Looper;
import android.util.Base64;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

/**
 * Runs via app_process as shell. One stdout line per app:
 * package \t base64(label) \t base64(signer O) \t base64(png)
 */
public final class ScanHelper {
    private static final int ICON_PX = 64;

    public static void main(String[] args) throws Exception {
        boolean thirdParty = args.length > 0 && "-3".equals(args[0]);
        Context context = systemContext();
        PackageManager pm = context.getPackageManager();
        List<ApplicationInfo> apps = new ArrayList<>();
        for (ApplicationInfo info : pm.getInstalledApplications(0)) {
            if (info.packageName == null) {
                continue;
            }
            if (thirdParty && (info.flags & ApplicationInfo.FLAG_SYSTEM) != 0) {
                continue;
            }
            apps.add(info);
        }
        if (apps.isEmpty()) {
            return;
        }
        int threads = Math.min(4, apps.size());
        ExecutorService pool = Executors.newFixedThreadPool(threads);
        for (ApplicationInfo info : apps) {
            pool.execute(new Emit(pm, info));
        }
        pool.shutdown();
        pool.awaitTermination(55, TimeUnit.SECONDS);
    }

    /** Same ActivityThread setup scrcpy uses so getSystemContext() works from app_process. */
    static Context systemContext() throws Exception {
        Looper.prepareMainLooper();
        Class<?> activityThreadClass = Class.forName("android.app.ActivityThread");
        Constructor<?> ctor = activityThreadClass.getDeclaredConstructor();
        ctor.setAccessible(true);
        Object activityThread = ctor.newInstance();
        Field current = activityThreadClass.getDeclaredField("sCurrentActivityThread");
        current.setAccessible(true);
        current.set(null, activityThread);
        Field systemThread = activityThreadClass.getDeclaredField("mSystemThread");
        systemThread.setAccessible(true);
        systemThread.setBoolean(activityThread, true);
        if (Build.VERSION.SDK_INT >= 31) {
            try {
                Class<?> controllerClass = Class.forName("android.app.ConfigurationController");
                Class<?> internalClass = Class.forName("android.app.ActivityThreadInternal");
                Constructor<?> controllerCtor = controllerClass.getDeclaredConstructor(internalClass);
                controllerCtor.setAccessible(true);
                Field field = activityThreadClass.getDeclaredField("mConfigurationController");
                field.setAccessible(true);
                field.set(activityThread, controllerCtor.newInstance(activityThread));
            } catch (Throwable ignored) {
                // Android 12+ only; PackageManager still works without it.
            }
        }
        // Without a bound application, getApplicationLabel/getApplicationIcon NPE on getResources().
        Class<?> bindClass = Class.forName("android.app.ActivityThread$AppBindData");
        Constructor<?> bindCtor = bindClass.getDeclaredConstructor();
        bindCtor.setAccessible(true);
        Object bind = bindCtor.newInstance();
        ApplicationInfo shell = new ApplicationInfo();
        shell.packageName = "com.android.shell";
        Field appInfo = bindClass.getDeclaredField("appInfo");
        appInfo.setAccessible(true);
        appInfo.set(bind, shell);
        Field bound = activityThreadClass.getDeclaredField("mBoundApplication");
        bound.setAccessible(true);
        bound.set(activityThread, bind);

        Method getSystemContext = activityThreadClass.getDeclaredMethod("getSystemContext");
        Context context = (Context) getSystemContext.invoke(activityThread);
        Application app = Instrumentation.newApplication(Application.class, context);
        Field initial = activityThreadClass.getDeclaredField("mInitialApplication");
        initial.setAccessible(true);
        initial.set(activityThread, app);
        return context;
    }

    static final class Emit implements Runnable {
        private final PackageManager pm;
        private final ApplicationInfo info;

        Emit(PackageManager pm, ApplicationInfo info) {
            this.pm = pm;
            this.info = info;
        }

        @Override
        public void run() {
            try {
                String label = "";
                CharSequence text = pm.getApplicationLabel(info);
                if (text != null) {
                    label = text.toString();
                }
                String png = info.icon == 0 ? "" : iconPng(pm, info);
                System.out.println(
                        info.packageName + "\t" + b64(label) + "\t" + b64(signerOrg(pm, info.packageName)) + "\t" + png);
            } catch (Throwable error) {
                System.err.println(info.packageName + " " + error);
            }
        }
    }

    static String signerOrg(PackageManager pm, String packageName) {
        try {
            Signature[] signatures = signatures(pm, packageName);
            if (signatures == null || signatures.length == 0) {
                return "";
            }
            CertificateFactory factory = CertificateFactory.getInstance("X.509");
            X509Certificate cert = (X509Certificate) factory.generateCertificate(
                    new ByteArrayInputStream(signatures[0].toByteArray()));
            String org = new SslCertificate(cert).getIssuedTo().getOName();
            return org == null ? "" : org;
        } catch (Throwable ignored) {
            return "";
        }
    }

    static Signature[] signatures(PackageManager pm, String packageName) throws Exception {
        if (Build.VERSION.SDK_INT >= 28) {
            SigningInfo signing = pm.getPackageInfo(packageName, PackageManager.GET_SIGNING_CERTIFICATES).signingInfo;
            if (signing == null) {
                return null;
            }
            return signing.hasMultipleSigners()
                    ? signing.getApkContentsSigners()
                    : signing.getSigningCertificateHistory();
        }
        return pm.getPackageInfo(packageName, PackageManager.GET_SIGNATURES).signatures;
    }

    static String iconPng(PackageManager pm, ApplicationInfo info) {
        Drawable drawable = pm.getApplicationIcon(info);
        Bitmap bitmap = Bitmap.createBitmap(ICON_PX, ICON_PX, Bitmap.Config.ARGB_8888);
        Canvas canvas = new Canvas(bitmap);
        drawable.setBounds(0, 0, ICON_PX, ICON_PX);
        drawable.draw(canvas);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)) {
            return "";
        }
        return Base64.encodeToString(out.toByteArray(), Base64.NO_WRAP);
    }

    static String b64(String value) {
        if (value == null || value.isEmpty()) {
            return "";
        }
        return Base64.encodeToString(value.getBytes(StandardCharsets.UTF_8), Base64.NO_WRAP);
    }
}
