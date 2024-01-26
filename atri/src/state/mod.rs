use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{bail, Result};
use atri_core::{search, MeiliClient, SearchResults};
use lru::LruCache;
use parking_lot::Mutex;

use crate::constants::PER_PAGE;

#[derive(Clone)]
pub struct Search {
  pub id: usize,
  pub keyword: String,
  pub offset: usize,
}

impl Search {
  fn prev_page(self) -> Self {
    Self {
      id: self.id,
      keyword: self.keyword,
      offset: self.offset - PER_PAGE,
    }
  }

  fn next_page(self) -> Self {
    Self {
      id: self.id,
      keyword: self.keyword,
      offset: self.offset + PER_PAGE,
    }
  }
}

#[derive(Debug)]
pub enum SearchMode {
  PrevPage,
  Direct,
  NextPage,
}

pub struct AtriState {
  pub count: AtomicUsize,
  pub searches: Mutex<LruCache<usize, Search>>,
  pub meili_client: MeiliClient,
}

impl AtriState {
  pub async fn new_search(&self, keyword: impl ToString) -> Result<(Search, SearchResults)> {
    let keyword = keyword.to_string();
    let games = search(&self.meili_client, &keyword, PER_PAGE + 1, 0).await?;

    let id = self.count.fetch_add(1, Ordering::Relaxed);

    let cur_search = {
      let search = Search {
        id,
        keyword,
        offset: 0,
      };
      let mut searches = self.searches.lock();
      searches.put(id, search.clone());
      search
    };

    Ok((cur_search, games))
  }

  pub async fn continue_search(
    &self,
    search_id: usize,
    mode: SearchMode,
  ) -> Result<(Search, SearchResults)> {
    let cur_search = {
      let mut searches = self.searches.lock();
      let Some(res) = searches.get(&search_id) else {
        bail!("Can not find search");
      };
      res.clone()
    };

    let games = search(
      &self.meili_client,
      &cur_search.keyword,
      PER_PAGE + 1,
      match mode {
        SearchMode::PrevPage => cur_search.offset - PER_PAGE,
        SearchMode::Direct => cur_search.offset,
        SearchMode::NextPage => cur_search.offset + PER_PAGE,
      },
    )
    .await?;

    let cur_search = match mode {
      SearchMode::Direct => cur_search,
      SearchMode::PrevPage => {
        let new_search = cur_search.prev_page();
        let mut searches = self.searches.lock();
        searches.put(search_id, new_search.clone());
        new_search
      }
      SearchMode::NextPage => {
        let new_search = cur_search.next_page();
        let mut searches = self.searches.lock();
        searches.put(search_id, new_search.clone());
        new_search
      }
    };

    Ok((cur_search, games))
  }

  pub fn drop_search(&self, search_id: usize) {
    let mut searches = self.searches.lock();
    searches.pop(&search_id);
  }
}
