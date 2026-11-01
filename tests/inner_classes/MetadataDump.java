import java.util.*;
import org.jetbrains.kotlin.metadata.deserialization.Flags;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;
public final class MetadataDump {
    public static void main(String[] args) throws Exception {
        for (String owner : args) {
            Class<?> type = Class.forName(owner, false, MetadataDump.class.getClassLoader());
            kotlin.Metadata metadata = type.getAnnotation(kotlin.Metadata.class);
            if (metadata == null || metadata.k() != 1) throw new AssertionError("class metadata absent");
            var decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
            var proto = decoded.getSecond();
            if (!Flags.IS_INNER.get(proto.getFlags())) throw new AssertionError("inner metadata absent");
            System.out.println(owner + " INNER=true");
            for (var constructor : proto.getConstructorList()) {
                var names = new ArrayList<String>();
                for (var parameter : constructor.getValueParameterList()) names.add(decoded.getFirst().getString(parameter.getName()));
                if (names.contains("this$0")) throw new AssertionError("synthetic receiver became value parameter");
                System.out.println(owner + " CONSTRUCTOR=" + names);
            }
            var names = new ArrayList<String>();
            for (var property : proto.getPropertyList()) names.add(decoded.getFirst().getString(property.getName()));
            Collections.sort(names);
            if (names.contains("this$0")) throw new AssertionError("synthetic receiver became property");
            System.out.println(owner + " PROPERTIES=" + names);
        }
    }
}
