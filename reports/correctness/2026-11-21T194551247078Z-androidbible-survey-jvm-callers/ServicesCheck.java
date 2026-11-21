import java.lang.reflect.Proxy;
import yuku.alkitab.base.services.*;

public class ServicesCheck {
    static <T> T unusedService(Class<T> type) {
        return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[]{type},
                (proxy, method, args) -> { throw new AssertionError(method); }));
    }
    public static void main(String[] args) {
        StorageProvider storage = unusedService(StorageProvider.class);
        VersionManager versions = unusedService(VersionManager.class);
        UiDimensionsProvider dimensions = unusedService(UiDimensionsProvider.class);
        AppServices services = new AppServices(storage, versions, dimensions);
        if (services.storage != storage || services.versions != versions || services.uiDimensions != dimensions)
            throw new AssertionError();
    }
}
