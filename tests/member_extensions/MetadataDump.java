import java.nio.file.*;
import java.util.*;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;
public final class MetadataDump {
    public static void main(String[] args) throws Exception {
        for (String owner : args) {
            Class<?> klass = Class.forName(owner, false, MetadataDump.class.getClassLoader());
            kotlin.Metadata metadata = klass.getAnnotation(kotlin.Metadata.class);
            if (metadata == null) throw new AssertionError("metadata absent: " + owner);
            if (metadata.k() == 1) {
                var decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
                for (var function : decoded.getSecond().getFunctionList()) {
                    System.out.println(owner+" FUNCTION "+decoded.getFirst().getString(function.getName())+" RECEIVER="+function.hasReceiverType()+" VALUES="+function.getValueParameterCount());
                }
                for (var property : decoded.getSecond().getPropertyList()) {
                    System.out.println(owner+" PROPERTY "+decoded.getFirst().getString(property.getName())+" RECEIVER="+property.hasReceiverType()+" FLAGS="+property.getFlags());
                }
            }
        }
    }
}
