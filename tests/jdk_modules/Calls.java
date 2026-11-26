package modules;
import com.sun.net.httpserver.HttpServer;
public class Calls {
    public static void main(String[] args) {
        HttpServer server = ExportsKt.http();
        if (!server.getAddress().getAddress().isLoopbackAddress()) throw new AssertionError();
        server.start();
        server.stop(0);
        if (ExportsKt.unsafe() != null) throw new AssertionError();
        if (ExportsKt.handler() != null) throw new AssertionError();
        if (ExportsKt.xml().getDocumentElement() != null) throw new AssertionError();
        if (ExportsKt.version() != java.net.http.HttpClient.Version.HTTP_1_1) throw new AssertionError();
        if (ExportsKt.driverVersion() != 0) throw new AssertionError();
        System.out.println("jdk-modules-ok");
    }
}
