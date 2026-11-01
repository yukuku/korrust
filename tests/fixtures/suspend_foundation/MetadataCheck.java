package suspension;

import java.util.ArrayList;
import java.util.Collections;
import kotlin.Metadata;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.deserialization.TypeTable;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;

public final class MetadataCheck {
    static String type(ProtoBuf.Type type, org.jetbrains.kotlin.metadata.jvm.deserialization.JvmNameResolver names, TypeTable table) {
        StringBuilder result = new StringBuilder();
        result.append(type.hasClassName() ? names.getQualifiedClassName(type.getClassName()) : "T" + type.getTypeParameter());
        result.append(" flags=").append(type.getFlags()).append(" nullable=").append(type.getNullable());
        for (ProtoBuf.Type.Argument argument : type.getArgumentList()) result.append(" [").append(argument.getProjection()).append(':').append(type(argument.hasType() ? argument.getType() : table.get(argument.getTypeId()), names, table)).append(']');
        return result.toString();
    }
    public static void main(String[] args) throws Exception {
        Metadata metadata = Class.forName("suspension.ModelsKt").getAnnotation(Metadata.class);
        var decoded = JvmProtoBufUtil.readPackageDataFrom(metadata.d1(), metadata.d2());
        var names = decoded.getFirst();
        var table = new TypeTable(decoded.getSecond().getTypeTable());
        ArrayList<String> functions = new ArrayList<>();
        for (ProtoBuf.Function function : decoded.getSecond().getFunctionList()) {
            StringBuilder result = new StringBuilder(names.getString(function.getName()));
            result.append(" flags=").append(function.getFlags()).append(" result=").append(type(function.hasReturnType() ? function.getReturnType() : table.get(function.getReturnTypeId()), names, table));
            for (ProtoBuf.ValueParameter parameter : function.getValueParameterList()) result.append(" PARAM ").append(names.getString(parameter.getName())).append(" flags=").append(parameter.getFlags()).append(" type=").append(type(parameter.hasType() ? parameter.getType() : table.get(parameter.getTypeId()), names, table));
            result.append(" JVM ").append(JvmProtoBufUtil.INSTANCE.getJvmMethodSignature(function, names, table).asString());
            functions.add(result.toString());
        }
        Collections.sort(functions);
        for (String function : functions) System.out.println(function);
    }
}
