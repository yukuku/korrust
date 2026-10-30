public class MetadataDump {
    public static void main(String[] names) throws Exception {
        for(String name:names) {
            kotlin.Metadata m=Class.forName(name).getAnnotation(kotlin.Metadata.class);
            var data=org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil.readClassDataFrom(m.d1(),m.d2());
            System.out.println(name);
            for(var f:data.getSecond().getFunctionList()) System.out.println("fun "+data.getFirst().getString(f.getName())+" flags "+f.getFlags());
            for(var p:data.getSecond().getPropertyList()) System.out.println("prop "+data.getFirst().getString(p.getName())+" flags "+p.getFlags()+" getter "+p.hasGetterFlags()+"/"+p.getGetterFlags()+" setter "+p.hasSetterFlags()+"/"+p.getSetterFlags());
        }
    }
}
