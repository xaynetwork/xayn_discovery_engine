// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'document.dart';

// **************************************************************************
// TypeAdapterGenerator
// **************************************************************************

class DocumentAdapter extends TypeAdapter<Document> {
  @override
  final int typeId = 0;

  @override
  Document read(BinaryReader reader) {
    final numOfFields = reader.readByte();
    final fields = <int, dynamic>{
      for (int i = 0; i < numOfFields; i++) reader.readByte(): reader.read(),
    };
    return Document(
      webResource: fields[1] as WebResource,
      nonPersonalizedRank: fields[4] as int,
      personalizedRank: fields[5] as int,
    );
  }

  @override
  void write(BinaryWriter writer, Document obj) {
    writer
      ..writeByte(6)
      ..writeByte(0)
      ..write(obj.documentId)
      ..writeByte(1)
      ..write(obj.webResource)
      ..writeByte(2)
      ..write(obj._feedback)
      ..writeByte(3)
      ..write(obj._status)
      ..writeByte(4)
      ..write(obj.nonPersonalizedRank)
      ..writeByte(5)
      ..write(obj.personalizedRank);
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DocumentAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}

class DocumentStatusAdapter extends TypeAdapter<DocumentStatus> {
  @override
  final int typeId = 1;

  @override
  DocumentStatus read(BinaryReader reader) {
    switch (reader.readByte()) {
      case 0:
        return DocumentStatus.skipped;
      case 1:
        return DocumentStatus.presented;
      case 2:
        return DocumentStatus.opened;
      case 3:
        return DocumentStatus.missed;
      default:
        return DocumentStatus.skipped;
    }
  }

  @override
  void write(BinaryWriter writer, DocumentStatus obj) {
    switch (obj) {
      case DocumentStatus.skipped:
        writer.writeByte(0);
        break;
      case DocumentStatus.presented:
        writer.writeByte(1);
        break;
      case DocumentStatus.opened:
        writer.writeByte(2);
        break;
      case DocumentStatus.missed:
        writer.writeByte(3);
        break;
    }
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DocumentStatusAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}

class DocumentFeedbackAdapter extends TypeAdapter<DocumentFeedback> {
  @override
  final int typeId = 2;

  @override
  DocumentFeedback read(BinaryReader reader) {
    switch (reader.readByte()) {
      case 0:
        return DocumentFeedback.neutral;
      case 1:
        return DocumentFeedback.positive;
      case 2:
        return DocumentFeedback.negative;
      default:
        return DocumentFeedback.neutral;
    }
  }

  @override
  void write(BinaryWriter writer, DocumentFeedback obj) {
    switch (obj) {
      case DocumentFeedback.neutral:
        writer.writeByte(0);
        break;
      case DocumentFeedback.positive:
        writer.writeByte(1);
        break;
      case DocumentFeedback.negative:
        writer.writeByte(2);
        break;
    }
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DocumentFeedbackAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}
