package members;

import java.util.ArrayList;
import java.util.Collections;
import kotlin.Metadata;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.deserialization.TypeTable;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;

public final class MetadataCheck {
    static String type(ProtoBuf.Type type, org.jetbrains.kotlin.metadata.jvm.deserialization.JvmNameResolver names, TypeTable table) {
        StringBuilder value = new StringBuilder(names.getQualifiedClassName(type.getClassName()));
        value.append('?').append(type.getNullable()).append(" flags=").append(type.getFlags());
        for (ProtoBuf.Type.Argument argument : type.getArgumentList()) value.append('[').append(argument.getProjection()).append(':').append(type(argument.hasType() ? argument.getType() : table.get(argument.getTypeId()), names, table)).append(']');
        return value.toString();
    }
    public static void main(String[] args) throws Exception {
        ArrayList<String> result = new ArrayList<>();
        for (String owner : new String[] {"Boundary", "Reader", "Immediate", "Scalars", "ExternalReader", "AbstractBoundary"}) {
            Metadata metadata = Class.forName("members." + owner).getAnnotation(Metadata.class);
            if (metadata == null) throw new AssertionError(owner);
            var decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
            var names = decoded.getFirst();
            var table = new TypeTable(decoded.getSecond().getTypeTable());
            for (ProtoBuf.Function function : decoded.getSecond().getFunctionList()) {
                if ((function.getFlags() & 8192) == 0) throw new AssertionError(owner + " lost suspend");
                StringBuilder value = new StringBuilder(owner + "." + names.getString(function.getName()));
                value.append(" result=").append(type(function.hasReturnType() ? function.getReturnType() : table.get(function.getReturnTypeId()), names, table));
                for (ProtoBuf.ValueParameter parameter : function.getValueParameterList()) value.append(" PARAM ").append(names.getString(parameter.getName())).append(':').append(type(parameter.hasType() ? parameter.getType() : table.get(parameter.getTypeId()), names, table));
                value.append(" JVM ").append(JvmProtoBufUtil.INSTANCE.getJvmMethodSignature(function, names, table).asString());
                result.add(value.toString());
            }
        }
        Collections.sort(result);
        for (String value : result) System.out.println(value);
    }
}
