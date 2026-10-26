import compatibility.annotations.Annotated;
import compatibility.annotations.SourceRetention;
import java.util.LinkedHashMap;
import java.util.Map;
import kotlin.Metadata;
import kotlin.Pair;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.deserialization.Flags;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmNameResolver;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;

public final class MetadataFlagsCheck {
    private static int annotated(String member, int flags) {
        boolean result = Flags.HAS_ANNOTATIONS.get(flags);
        if (!result) throw new AssertionError(member + " missing HAS_ANNOTATIONS, flags=" + flags);
        return flags;
    }

    private static int unannotated(String member, int flags) {
        if (Flags.HAS_ANNOTATIONS.get(flags)) throw new AssertionError(member + " retained SOURCE metadata flags=" + flags);
        return flags;
    }

    private static void sourceRetention(Map<String, Integer> snapshot) {
        Metadata metadata = SourceRetention.class.getAnnotation(Metadata.class);
        if (metadata == null) throw new AssertionError("SOURCE fixture Kotlin Metadata missing");
        Pair<JvmNameResolver, ProtoBuf.Class> decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
        JvmNameResolver names = decoded.getFirst();
        ProtoBuf.Class declaration = decoded.getSecond();
        snapshot.put("source_class", unannotated("SOURCE class", declaration.getFlags()));
        ProtoBuf.Function plus = null;
        for (ProtoBuf.Function function : declaration.getFunctionList()) {
            if (names.getString(function.getName()).equals("plus")) plus = function;
        }
        if (plus == null) throw new AssertionError("SOURCE plus metadata missing");
        snapshot.put("source_plus", unannotated("SOURCE plus", plus.getFlags()));
        if (declaration.getConstructorCount() != 1 || declaration.getConstructor(0).getValueParameterCount() != 1) {
            throw new AssertionError("SOURCE primary constructor metadata missing");
        }
        snapshot.put("source_constructor_value", unannotated("SOURCE constructor value", declaration.getConstructor(0).getValueParameter(0).getFlags()));
        ProtoBuf.Property value = null;
        for (ProtoBuf.Property property : declaration.getPropertyList()) {
            if (names.getString(property.getName()).equals("value")) value = property;
        }
        if (value == null) throw new AssertionError("SOURCE property metadata missing");
        snapshot.put("source_property", unannotated("SOURCE property", value.getFlags()));
        int defaults = Flags.getAccessorFlags(Flags.HAS_ANNOTATIONS.get(value.getFlags()),
            Flags.VISIBILITY.get(value.getFlags()), Flags.MODALITY.get(value.getFlags()), false, false, false);
        snapshot.put("source_getter", unannotated("SOURCE getter", value.hasGetterFlags() ? value.getGetterFlags() : defaults));
        snapshot.put("source_setter", unannotated("SOURCE setter", value.hasSetterFlags() ? value.getSetterFlags() : defaults));
        snapshot.put("source_setter_parameter", unannotated("SOURCE setter parameter", value.getSetterValueParameter().getFlags()));
    }

    public static void main(String[] args) {
        Metadata metadata = Annotated.class.getAnnotation(Metadata.class);
        if (metadata == null) throw new AssertionError("Kotlin Metadata missing");
        Pair<JvmNameResolver, ProtoBuf.Class> decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
        JvmNameResolver names = decoded.getFirst();
        ProtoBuf.Class declaration = decoded.getSecond();
        Map<String, Integer> snapshot = new LinkedHashMap<>();
        snapshot.put("class", annotated("Annotated", declaration.getFlags()));
        ProtoBuf.Function plus = null;
        for (ProtoBuf.Function function : declaration.getFunctionList()) {
            if (names.getString(function.getName()).equals("plus")) {
                if (plus != null) throw new AssertionError("Duplicate plus metadata");
                plus = function;
            }
        }
        if (plus == null) throw new AssertionError("plus metadata missing");
        snapshot.put("plus", annotated("plus", plus.getFlags()));
        ProtoBuf.Constructor primary = null;
        for (ProtoBuf.Constructor constructor : declaration.getConstructorList()) {
            if (!Flags.IS_SECONDARY.get(constructor.getFlags())) {
                if (primary != null) throw new AssertionError("Duplicate primary constructor metadata");
                primary = constructor;
            }
        }
        if (primary == null || primary.getValueParameterCount() != 1) throw new AssertionError("Primary constructor metadata missing");
        ProtoBuf.ValueParameter parameter = primary.getValueParameter(0);
        if (!names.getString(parameter.getName()).equals("value")) throw new AssertionError("Constructor parameter identity");
        snapshot.put("constructor_value", annotated("constructor value", parameter.getFlags()));
        ProtoBuf.Property value = null;
        for (ProtoBuf.Property property : declaration.getPropertyList()) {
            if (names.getString(property.getName()).equals("value")) {
                if (value != null) throw new AssertionError("Duplicate value property metadata");
                value = property;
            }
        }
        if (value == null || !value.hasSetterValueParameter()) {
            throw new AssertionError("Property/setter parameter metadata missing");
        }
        annotated("value property backing-field annotations", value.getFlags());
        int defaultAccessorFlags = Flags.getAccessorFlags(
            Flags.HAS_ANNOTATIONS.get(value.getFlags()), Flags.VISIBILITY.get(value.getFlags()),
            Flags.MODALITY.get(value.getFlags()), false, false, false);
        int getterFlags = value.hasGetterFlags() ? value.getGetterFlags() : defaultAccessorFlags;
        int setterFlags = value.hasSetterFlags() ? value.getSetterFlags() : defaultAccessorFlags;
        snapshot.put("getter", annotated("value getter", getterFlags));
        snapshot.put("setter", annotated("value setter", setterFlags));
        snapshot.put("setter_parameter", annotated("value setter parameter", value.getSetterValueParameter().getFlags()));
        sourceRetention(snapshot);
        StringBuilder json = new StringBuilder("{");
        for (Map.Entry<String, Integer> entry : snapshot.entrySet()) {
            if (json.length() > 1) json.append(',');
            json.append('"').append(entry.getKey()).append("\":").append(entry.getValue());
        }
        System.out.println(json.append('}'));
    }
}
