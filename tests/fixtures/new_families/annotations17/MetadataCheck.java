package annotation17check;

import annotation17.*;
import kotlin.Metadata;
import kotlin.Pair;
import org.jetbrains.kotlin.metadata.ProtoBuf;
import org.jetbrains.kotlin.metadata.deserialization.Flags;
import org.jetbrains.kotlin.metadata.jvm.JvmProtoBuf;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmNameResolver;
import org.jetbrains.kotlin.metadata.jvm.deserialization.JvmProtoBufUtil;

public final class MetadataCheck {
    private static int count;
    private static void check(boolean value) {
        count++;
        if (!value) throw new AssertionError("metadata check " + count);
    }
    public static void main(String[] args) {
        Metadata metadata = Rich.class.getAnnotation(Metadata.class);
        check(metadata != null && metadata.k() == 1);
        Pair<JvmNameResolver, ProtoBuf.Class> decoded = JvmProtoBufUtil.readClassDataFrom(metadata.d1(), metadata.d2());
        JvmNameResolver names = decoded.getFirst();
        ProtoBuf.Class declaration = decoded.getSecond();
        check(Flags.CLASS_KIND.get(declaration.getFlags()) == ProtoBuf.Class.Kind.ANNOTATION_CLASS);
        check(Flags.MODALITY.get(declaration.getFlags()) == ProtoBuf.Modality.FINAL);
        check(names.getQualifiedClassName(declaration.getFqName()).equals("annotation17/Rich"));
        check(declaration.getConstructorCount() == 1);
        ProtoBuf.Constructor constructor = declaration.getConstructor(0);
        String[] expected = {"number", "ratio", "measure", "letter", "mode", "type", "numbers", "flags", "letters", "ratios", "measures", "names", "modes", "types", "nested", "children"};
        ProtoBuf.Annotation.Argument.Value.Type[] kinds = {
            ProtoBuf.Annotation.Argument.Value.Type.LONG, ProtoBuf.Annotation.Argument.Value.Type.FLOAT,
            ProtoBuf.Annotation.Argument.Value.Type.DOUBLE, ProtoBuf.Annotation.Argument.Value.Type.CHAR,
            ProtoBuf.Annotation.Argument.Value.Type.ENUM, ProtoBuf.Annotation.Argument.Value.Type.CLASS,
            ProtoBuf.Annotation.Argument.Value.Type.ARRAY, ProtoBuf.Annotation.Argument.Value.Type.ARRAY,
            ProtoBuf.Annotation.Argument.Value.Type.ARRAY, ProtoBuf.Annotation.Argument.Value.Type.ARRAY,
            ProtoBuf.Annotation.Argument.Value.Type.ARRAY, ProtoBuf.Annotation.Argument.Value.Type.ARRAY,
            ProtoBuf.Annotation.Argument.Value.Type.ARRAY, ProtoBuf.Annotation.Argument.Value.Type.ARRAY,
            ProtoBuf.Annotation.Argument.Value.Type.ANNOTATION, ProtoBuf.Annotation.Argument.Value.Type.ARRAY
        };
        check(constructor.getValueParameterCount() == expected.length);
        for (int i = 0; i < expected.length; i++) {
            ProtoBuf.ValueParameter parameter = constructor.getValueParameter(i);
            check(names.getString(parameter.getName()).equals(expected[i]));
            check(Flags.DECLARES_DEFAULT_VALUE.get(parameter.getFlags()));
            check(parameter.hasAnnotationParameterDefaultValue());
            check(parameter.getAnnotationParameterDefaultValue().getType() == kinds[i]);
        }
        check(constructor.getValueParameter(0).getAnnotationParameterDefaultValue().getIntValue() == Long.MIN_VALUE);
        check(Float.floatToRawIntBits(constructor.getValueParameter(1).getAnnotationParameterDefaultValue().getFloatValue()) == 0x80000000);
        check(constructor.getValueParameter(2).getAnnotationParameterDefaultValue().getDoubleValue() == 1.25);
        check(constructor.getValueParameter(3).getAnnotationParameterDefaultValue().getIntValue() == 0xD800);
        ProtoBuf.Annotation.Argument.Value mode = constructor.getValueParameter(4).getAnnotationParameterDefaultValue();
        check(names.getQualifiedClassName(mode.getClassId()).equals("annotation17/Mode"));
        check(names.getString(mode.getEnumValueId()).equals("SECOND"));
        ProtoBuf.Annotation.Argument.Value type = constructor.getValueParameter(5).getAnnotationParameterDefaultValue();
        check(names.getQualifiedClassName(type.getClassId()).equals("annotation17/Token") && type.getArrayDimensionCount() == 0);
        ProtoBuf.Annotation.Argument.Value numbers = constructor.getValueParameter(6).getAnnotationParameterDefaultValue();
        check(numbers.getArrayElementCount() == 3);
        check(numbers.getArrayElement(0).getIntValue() == 0L && numbers.getArrayElement(1).getIntValue() == 1L && numbers.getArrayElement(2).getIntValue() == -1L);
        ProtoBuf.Annotation.Argument.Value flags = constructor.getValueParameter(7).getAnnotationParameterDefaultValue();
        check(flags.getArrayElementCount() == 2 && flags.getArrayElement(0).getIntValue() == 1L && flags.getArrayElement(1).getIntValue() == 0L);
        ProtoBuf.Annotation.Argument.Value letters = constructor.getValueParameter(8).getAnnotationParameterDefaultValue();
        check(letters.getArrayElementCount() == 3 && letters.getArrayElement(1).getIntValue() == 0xD800 && letters.getArrayElement(2).getIntValue() == 0xDFFF);
        ProtoBuf.Annotation.Argument.Value ratios = constructor.getValueParameter(9).getAnnotationParameterDefaultValue();
        check(ratios.getArrayElementCount() == 2 && Float.floatToRawIntBits(ratios.getArrayElement(0).getFloatValue()) == 0x80000000);
        ProtoBuf.Annotation.Argument.Value measures = constructor.getValueParameter(10).getAnnotationParameterDefaultValue();
        check(measures.getArrayElementCount() == 2 && Double.doubleToRawLongBits(measures.getArrayElement(0).getDoubleValue()) == Long.MIN_VALUE);
        ProtoBuf.Annotation.Argument.Value strings = constructor.getValueParameter(11).getAnnotationParameterDefaultValue();
        check(strings.getArrayElementCount() == 4);
        check(names.getString(strings.getArrayElement(1).getStringValue()).equals("\u0000"));
        check(names.getString(strings.getArrayElement(2).getStringValue()).equals("\uD800"));
        check(names.getString(strings.getArrayElement(3).getStringValue()).equals("\uD83D\uDE00"));
        ProtoBuf.Annotation.Argument.Value modes = constructor.getValueParameter(12).getAnnotationParameterDefaultValue();
        check(modes.getArrayElementCount() == 2);
        check(names.getString(modes.getArrayElement(0).getEnumValueId()).equals("FIRST") && names.getString(modes.getArrayElement(1).getEnumValueId()).equals("SECOND"));
        ProtoBuf.Annotation.Argument.Value types = constructor.getValueParameter(13).getAnnotationParameterDefaultValue();
        check(types.getArrayElementCount() == 5);
        check(names.getQualifiedClassName(types.getArrayElement(0).getClassId()).equals("kotlin/Int"));
        check(names.getQualifiedClassName(types.getArrayElement(1).getClassId()).equals("kotlin/String") && types.getArrayElement(1).getArrayDimensionCount() == 1);
        check(names.getQualifiedClassName(types.getArrayElement(2).getClassId()).equals("kotlin/IntArray") && types.getArrayElement(2).getArrayDimensionCount() == 0);
        ProtoBuf.Annotation nested = constructor.getValueParameter(14).getAnnotationParameterDefaultValue().getAnnotation();
        check(names.getQualifiedClassName(nested.getId()).equals("annotation17/Nested") && nested.getArgumentCount() == 0);
        ProtoBuf.Annotation.Argument.Value children = constructor.getValueParameter(15).getAnnotationParameterDefaultValue();
        check(children.getArrayElementCount() == 2);
        check(children.getArrayElement(0).getAnnotation().getArgumentCount() == 3 && children.getArrayElement(1).getAnnotation().getArgumentCount() == 0);
        check(declaration.getPropertyCount() == expected.length);
        for (ProtoBuf.Property property : declaration.getPropertyList()) {
            check(!Flags.IS_VAR.get(property.getFlags()));
            JvmProtoBuf.JvmPropertySignature signature = property.getExtension(JvmProtoBuf.propertySignature);
            check(signature.hasGetter());
            check(names.getString(signature.getGetter().getName()).equals(names.getString(property.getName())));
            check(!signature.hasField() && !signature.hasSetter());
        }
        System.out.println("OK:" + count);
    }
}
