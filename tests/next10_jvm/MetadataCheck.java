import kotlin.Metadata;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.deserialization.Flags;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;

public class MetadataCheck {
    static void check(String name, ProtoBuf.Visibility expected) throws Exception {
        Metadata metadata = Class.forName(name).getAnnotation(Metadata.class);
        ProtoBuf.Class klass = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2()).getSecond();
        if (klass.getConstructorCount() != 1) throw new AssertionError("constructor count");
        ProtoBuf.Constructor ctor = klass.getConstructor(0);
        if (Flags.VISIBILITY.get(ctor.getFlags()) != expected) throw new AssertionError(name + " visibility");
    }
    public static void main(String[] args) throws Exception {
        check("nextjvm.api.Secret", ProtoBuf.Visibility.PRIVATE);
        check("nextjvm.api.ObjectSecret", ProtoBuf.Visibility.PRIVATE);
        check("nextjvm.api.ModuleOnly", ProtoBuf.Visibility.INTERNAL);
        Metadata metadata = Class.forName("nextjvm.api.StableApi").getAnnotation(Metadata.class);
        if (metadata.k() != 2) throw new AssertionError("file facade metadata");
        System.out.println("METADATA");
    }
}
