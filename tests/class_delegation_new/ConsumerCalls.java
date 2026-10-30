import delegated.*;
public class ConsumerCalls {
    public static void main(String[] args){
        Calls.Ints ints=new Calls.Ints();PlainIntForward i=new PlainIntForward(ints);
        if(ConsumerKt.scalarConsumer(i)!=1000)throw new AssertionError();
        ints.stored=null;ints.effects=0;
        try{ConsumerKt.scalarConsumer(i);throw new AssertionError();}catch(NullPointerException e){}
        if(ints.effects!=1)throw new AssertionError();
        Calls.Refs refs=new Calls.Refs();RefForward ref=new RefForward(refs);
        if(ConsumerKt.stringConsumer(ref)!=null)throw new AssertionError();
        refs.stored="OK";if(!ConsumerKt.genericConsumer(new Forward<String>(refs)).equals("OK"))throw new AssertionError();
        if(!ConsumerKt.inheritedConsumer(new Inherited<String>((value)->value+"K")).equals("OK"))throw new AssertionError();
        if(ConsumerKt.counterConsumer(new Saved(new Calls.Counts(1,"n")))!=26)throw new AssertionError();
        Calls.JavaTexts javaText=new Calls.JavaTexts();
        if(ConsumerKt.platformMetadataConsumer(new JavaForward(javaText))!=null || javaText.effects!=1)throw new AssertionError();
        Calls.Values<Float> floats=new Calls.Values<Float>(-0.0f);
        if(Float.floatToRawIntBits(ConsumerKt.floatConsumer(new FloatForward(floats)))!=Float.floatToRawIntBits(-0.0f))throw new AssertionError("Float consumer");
        Calls.Values<Double> doubles=new Calls.Values<Double>(-0.0);
        if(Double.doubleToRawLongBits(ConsumerKt.doubleConsumer(new DoubleForward(doubles)))!=Double.doubleToRawLongBits(-0.0))throw new AssertionError("Double consumer");
        if(ConsumerKt.byteConsumer(new ByteForward(new Calls.Values<Byte>((byte)-128)))!=(byte)-128)throw new AssertionError("Byte consumer");
        System.out.println("OK");
    }
}
