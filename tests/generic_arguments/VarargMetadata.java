import genericargs.Varargs;
import kotlin.Metadata;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;
public class VarargMetadata {
    public static void main(String[] args) {
        Metadata metadata = Varargs.class.getAnnotation(Metadata.class);
        var decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
        var names = decoded.getFirst();
        var klass = decoded.getSecond();
        if (klass.getTypeParameterCount() != 1 || klass.getTypeParameter(0).getId() != 0) throw new AssertionError("class parameter");
        int projected = 0;
        int primitives = 0;
        for (var method : klass.getFunctionList()) {
            String name = names.getString(method.getName());
            if (method.getTypeParameterCount() != 1 || method.getTypeParameter(0).getId() != 1) throw new AssertionError("method parameter " + name);
            var parameter = method.getValueParameter(method.getValueParameterCount() - 1);
            if (!parameter.hasVarargElementType()) throw new AssertionError("vararg element " + name);
            if (name.equals("primitive")) {
                primitives++;
                continue;
            }
            var type = parameter.getType();
            if (type.getArgumentCount() != 1 || type.getArgument(0).getProjection() != ProtoBuf.Type.Argument.Projection.OUT) throw new AssertionError("out array " + name);
            int expected = name.equals("classifier") ? 0 : 1;
            for (var element : new ProtoBuf.Type[]{parameter.getVarargElementType(), type.getArgument(0).getType()}) {
                boolean matches = element.hasTypeParameter()
                    ? element.getTypeParameter() == expected
                    : element.hasTypeParameterName() && names.getString(element.getTypeParameterName()).equals(expected == 0 ? "C" : "T");
                if (!matches) throw new AssertionError("element parameter " + name);
            }
            projected++;
        }
        if (projected != 7 || primitives != 1) throw new AssertionError("method counts " + projected + "/" + primitives);
        System.out.println("OK");
    }
}
