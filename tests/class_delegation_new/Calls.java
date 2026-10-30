import delegated.*;
public class Calls {
    static final class Ints implements Contract<Integer> {
        Integer stored=new Integer(1000); Integer lastInput; int effects;
        public Integer echo(Integer value){effects++;lastInput=value;return stored;}
        public Integer getValue(){effects++;return stored;}
        public void setValue(Integer v){effects++;stored=v;}
        public String getLabel(){return "label";}
    }
    static final class Refs implements Contract<String> {
        String stored; int effects;
        public String echo(String v){effects++;return stored;}
        public String getValue(){effects++;return stored;}
        public void setValue(String v){effects++;stored=v;}
        public String getLabel(){return "label";}
    }
    static final class Values<T> implements Contract<T> {
        T stored;T last;int effects;
        Values(T value){stored=value;}
        public T echo(T value){effects++;last=value;return stored;}
        public T getValue(){effects++;return stored;}
        public void setValue(T value){effects++;stored=value;}
        public String getLabel(){return "value";}
    }
    static final class Counts implements Counter {
        int count; final String name;
        Counts(int count,String name){this.count=count;this.name=name;}
        public int getCount(){return count;}
        public void setCount(int value){count=value;}
        public String getName(){return name;}
        public int read(){return count;}
        public long twice(long value){return value+read();}
    }
    static final class Ones implements A { public int one(int value){return value+5;} }
    static final class LeftImpl implements Left { public int value(){return 1;} }
    static final class RightImpl implements Right { public int value(){return 2;} }
    static final class Bs implements B { int count;public int getCount(){return count;}public void setCount(int v){count=v;} }
    static final class JavaTexts implements JavaText {
        int effects; Object value;
        public String echo(String value){effects++;return value;}
        public Object getValue(){effects++;return value;}
        public void setValue(Object value){effects++;this.value=value;}
    }
    public static void main(String[] args){
        Ints i=new Ints();PlainIntForward p=new PlainIntForward(i);Integer expected=i.stored;
        if(p.echo(7)!=expected || p.getValue()!=expected)throw new AssertionError("identity");
        i.stored=null;if(p.echo(7)!=null || p.getValue()!=null)throw new AssertionError("null result");
        Contract raw=p;i.effects=0;
        try{raw.echo(null);throw new AssertionError();}catch(NullPointerException e){}
        try{raw.echo("wrong");throw new AssertionError();}catch(ClassCastException e){}
        if(i.effects!=0)throw new AssertionError("bridge effects");
        raw.setValue(1001);if(p.getValue()!=1001)throw new AssertionError();
        Forward<Integer> generic=new Forward<>(i);if(generic.echo(null)!=i.stored)throw new AssertionError();
        Refs r=new Refs();RefForward ref=new RefForward(r);if(ref.getValue()!=null || ref.echo("x")!=null)throw new AssertionError();
        r.effects=0;try{ref.echo(null);throw new AssertionError();}catch(NullPointerException e){}
        if(r.effects!=0)throw new AssertionError("nonnull effects");
        Counts first=new Counts(2,"first"), second=new Counts(9,"second");
        Events.trace="";Saved saved=new Saved(first);
        if(!Events.trace.equals("134") || saved.getInitialized()!=3)throw new AssertionError("initialization "+Events.trace);
        saved.setCurrent(second);saved.setCount(7);
        if(saved.getCurrent()!=second || saved.getCount()!=7 || first.count!=7 || second.count!=9 || !saved.getName().equals("first") || saved.read()!=7 || saved.twice(0x100000001L)!=0x100000008L || !Events.trace.equals("134"))throw new AssertionError("saved receiver");
        OverrideCounter override=new OverrideCounter(first);
        if(override.getCount()!=11 || override.read()!=99 || override.twice(1L)!=8L)throw new AssertionError("override precedence");
        override.setCount(12);if(override.getCount()!=12 || first.count!=7)throw new AssertionError("property override");
        Events.trace="";Bs bs=new Bs();Ordered ordered=new Ordered(new Ones(),bs);
        if(!Events.trace.equals("1234") || ordered.one(7)!=12 || ordered.getInitialized()!=3)throw new AssertionError("multiple delegates "+Events.trace);
        ordered.setCount(17);if(ordered.getCount()!=17 || bs.count!=17)throw new AssertionError();
        Child<String> child=new Inherited<String>((value)->value+"K");if(!child.echo("O").equals("OK"))throw new AssertionError("inherited generic");
        Events.trace="";try{new Saved(null);throw new AssertionError();}catch(NullPointerException e){}
        if(!Events.trace.isEmpty())throw new AssertionError("null constructor effects");
        if(new Collision(first).read()!=7 || new Shared(new LeftImpl(),new RightImpl()).value()!=77)throw new AssertionError();
        if(UseKt.scalar(new PlainIntForward(new Ints()))!=1000 || UseKt.stringValue(ref)!=null)throw new AssertionError("source callers");
        Events.trace="";Caught caught=new Caught(first);
        if(caught.read()!=7 || !Events.trace.equals("5"))throw new AssertionError("caught initializer");
        JavaTexts jt=new JavaTexts();JavaForward jf=new JavaForward(jt);
        if(jf.echo(null)!=null || jt.effects!=1 || UseKt.platformConsumer(jf)!=null || jt.effects!=2)throw new AssertionError("Java platform parameters");
        Object identity=new Object();jf.setValue(identity);if(jf.getValue()!=identity)throw new AssertionError("Java platform result");
        if(new Derived(first).read()!=7)throw new AssertionError("inherited forwarders");
        i.stored=expected;i.effects=0;raw.echo(2.9);if(i.lastInput!=2 || i.effects!=1)throw new AssertionError("numeric bridge conversion");
        Values<Long> longs=new Values<Long>(new Long(0x100000001L));LongForward lf=new LongForward(longs);
        if(lf.echo(9L)!=longs.stored || lf.getValue()!=longs.stored)throw new AssertionError("wide boxed identity");
        ((Contract)lf).echo(7.9);if(longs.last!=7L)throw new AssertionError("wide bridge conversion");
        longs.stored=null;if(lf.getValue()!=null)throw new AssertionError();
        Values<Boolean> booleans=new Values<Boolean>(new Boolean(true));BooleanForward bf=new BooleanForward(booleans);
        if(bf.echo(false)!=booleans.stored)throw new AssertionError("Boolean identity");
        booleans.effects=0;try{((Contract)bf).echo(1);throw new AssertionError();}catch(ClassCastException e){}
        if(booleans.effects!=0)throw new AssertionError();
        Values<Character> chars=new Values<Character>(new Character('\uD800'));CharForward cf=new CharForward(chars);
        if(cf.echo('x')!=chars.stored || cf.getValue().charValue()!='\uD800')throw new AssertionError("UTF16 character identity");
        Values<Integer> nullable=new Values<Integer>(null);NullableForward nf=new NullableForward(nullable);
        if(nf.echo(null)!=null || nullable.effects!=1)throw new AssertionError("nullable primary parameter");
        ((Contract)nf).echo(null);if(nullable.effects!=2 || nullable.last!=null)throw new AssertionError("nullable bridge parameter");
        r.stored="\u0000\uD800\uDC00\uDFFF";if(ref.echo("input")!=r.stored)throw new AssertionError("UTF16 reference identity");
        RuntimeException failure=new IllegalStateException("initializer");Events.failure=failure;Events.failAt=2;Events.trace="";
        try{new Ordered(new Ones(),new Bs());throw new AssertionError();}catch(RuntimeException e){if(e!=failure)throw new AssertionError();}
        if(!Events.trace.equals("12"))throw new AssertionError("exception order "+Events.trace);
        Events.failure=null;Events.failAt=0;
        Values<Float> floats=new Values<Float>(Float.valueOf(Float.intBitsToFloat(0x7fc01234)));FloatForward ff=new FloatForward(floats);
        if(ff.echo(-0.0f)!=floats.stored || ff.getValue()!=floats.stored || Float.floatToRawIntBits(ff.getValue())!=0x7fc01234)throw new AssertionError("Float identity and NaN payload");
        ((Contract)ff).echo(1.9);if(Float.floatToRawIntBits(floats.last)!=Float.floatToRawIntBits((float)1.9))throw new AssertionError("Float bridge conversion");
        Values<Double> doubles=new Values<Double>(Double.valueOf(Double.longBitsToDouble(0x7ff8000000001234L)));DoubleForward df=new DoubleForward(doubles);
        if(df.echo(-0.0)!=doubles.stored || df.getValue()!=doubles.stored || Double.doubleToRawLongBits(df.getValue())!=0x7ff8000000001234L)throw new AssertionError("Double identity and NaN payload");
        ((Contract)df).echo(-0.0f);if(Double.doubleToRawLongBits(doubles.last)!=Double.doubleToRawLongBits(-0.0))throw new AssertionError("Double signed-zero bridge");
        Values<Byte> bytes=new Values<Byte>(Byte.valueOf((byte)-128));ByteForward byteForward=new ByteForward(bytes);
        if(byteForward.echo((byte)127)!=bytes.stored || byteForward.getValue()!=bytes.stored)throw new AssertionError("Byte identity");
        ((Contract)byteForward).echo(257);if(bytes.last!=(byte)1)throw new AssertionError("Byte bridge narrowing");
        Diamond diamond=()->23;if(new DiamondForward(diamond).shared()!=23)throw new AssertionError("abstract diamond");
        System.out.println("OK");
    }
}
