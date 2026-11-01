package enums.polymorphism;

import kotlin.Metadata;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;
import java.util.ArrayList;
import java.util.Collections;

public class MetadataCheck {
    public static void main(String[] args) throws Exception {
        for (String name : args) {
            Metadata metadata = Class.forName(name).getAnnotation(Metadata.class);
            var decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
            var resolver = decoded.getFirst();
            var proto = decoded.getSecond();
            System.out.println(name + " class=" + proto.getFlags() + " id=" + resolver.getQualifiedClassName(proto.getFqName()));
            ArrayList<String> members = new ArrayList<>();
            for (ProtoBuf.Type parent : proto.getSupertypeList()) members.add("super=" + resolver.getQualifiedClassName(parent.getClassName()));
            for (ProtoBuf.EnumEntry entry : proto.getEnumEntryList()) members.add("entry=" + resolver.getString(entry.getName()));
            for (ProtoBuf.Function method : proto.getFunctionList()) members.add("function=" + resolver.getString(method.getName()) + ":" + method.getFlags());
            for (ProtoBuf.Property property : proto.getPropertyList()) members.add("property=" + resolver.getString(property.getName()) + ":" + property.getFlags());
            for (ProtoBuf.Constructor constructor : proto.getConstructorList()) {
                StringBuilder parameters = new StringBuilder();
                for (ProtoBuf.ValueParameter parameter : constructor.getValueParameterList()) parameters.append(resolver.getString(parameter.getName())).append(':').append(parameter.getFlags()).append(';');
                members.add("constructor=" + constructor.getFlags() + ":" + parameters);
            }
            Collections.sort(members);
            for (String member : members) System.out.println(member);
        }
    }
}
