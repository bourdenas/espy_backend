#include "util/proto_util.hpp"

namespace espy {
namespace util {

void SaveProto(const google::protobuf::Message& message,
               const std::string& filename_base) {
  std::ofstream dbg_file;
  dbg_file.open(absl::StrCat(filename_base, ".txt"), std::ios::out);
  dbg_file << message.DebugString();
  dbg_file.close();

  std::ofstream bin_file;
  bin_file.open(absl::StrCat(filename_base, ".bin"),
                std::ios::out | std::ios::binary);
  bin_file << message.SerializeAsString();
  bin_file.close();
}

}  // namespace util
}  // namespace espy
