import kotlin.Metadata;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.jvm.JvmProtoBuf;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;
public final class MetadataDump {
    public static void main(String[] args) throws Exception {
        for (String owner : args) {
            Metadata metadata = Class.forName(owner, false, MetadataDump.class.getClassLoader()).getAnnotation(Metadata.class);
            if (metadata == null) throw new AssertionError("metadata absent " + owner);
            var data = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
            var names = data.getFirst();
            ProtoBuf.Class declaration = data.getSecond();
            System.out.println(owner + " FLAGS " + declaration.getFlags() + " PROPERTY " + names.getString(declaration.getInlineClassUnderlyingPropertyName()));
            ProtoBuf.Type carrier = null;
            if (declaration.hasInlineClassUnderlyingType()) carrier = declaration.getInlineClassUnderlyingType();
            else if (declaration.hasInlineClassUnderlyingTypeId()) carrier = declaration.getTypeTable().getType(declaration.getInlineClassUnderlyingTypeId());
            else for (var property : declaration.getPropertyList()) if (property.getName() == declaration.getInlineClassUnderlyingPropertyName()) carrier = property.hasReturnType() ? property.getReturnType() : declaration.getTypeTable().getType(property.getReturnTypeId());
            if (carrier == null) throw new AssertionError("carrier metadata absent " + owner);
            System.out.println(owner + " CARRIER " + names.getQualifiedClassName(carrier.getClassName()) + " NULLABLE " + carrier.getNullable());
            for (var constructor : declaration.getConstructorList()) {
                var signature = constructor.getExtension(JvmProtoBuf.constructorSignature);
                System.out.println(owner + " CONSTRUCTOR " + names.getString(signature.getName()) + names.getString(signature.getDesc()));
            }
        }
    }
}
