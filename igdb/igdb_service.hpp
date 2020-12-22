#ifndef ESPY_SERVER_IGDB_IGDB_SERVICE_HPP_
#define ESPY_SERVER_IGDB_IGDB_SERVICE_HPP_

#include <string>
#include <string_view>

#include <absl/status/statusor.h>

#include "proto/search_result.pb.h"

namespace espy {

class IgdbService {
 public:
  IgdbService(std::string client_id, std::string secret)
      : client_id_(std::move(client_id)), secret_(secret) {}
  virtual ~IgdbService() = default;

  // Returns search result candidates from IGDB server with entries that
  // match input title.
  virtual absl::StatusOr<igdb::SearchResultList> SearchByTitle(
      std::string_view title) const;

 private:
  const std::string client_id_;
  const std::string secret_;

  mutable std::string oauth_token_;
};

}  // namespace espy

#endif  // ESPY_SERVER_IGDB_IGDB_SERVICE_HPP_
