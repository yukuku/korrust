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
            var visibility = Flags.VISIBILITY.get(proto.getFlags());
            if (!visibility.name().equals("PRIVATE")) throw new AssertionError(visibility);
            System.out.println(owner + " VISIBILITY=" + visibility + " DATA=" + Flags.IS_DATA.get(proto.getFlags()) + " INNER=" + Flags.IS_INNER.get(proto.getFlags()));
            for (var constructor : proto.getConstructorList()) {
                var names = new ArrayList<String>();
                for (var parameter : constructor.getValueParameterList()) names.add(decoded.getFirst().getString(parameter.getName()));
                System.out.println(owner + " CONSTRUCTOR=" + Flags.VISIBILITY.get(constructor.getFlags()) + names);
            }
            var names = new ArrayList<String>();
            for (var property : proto.getPropertyList()) names.add(decoded.getFirst().getString(property.getName()) + ":" + Flags.VISIBILITY.get(property.getFlags()));
            Collections.sort(names);
            System.out.println(owner + " PROPERTIES=" + names);
        }
    }
}
